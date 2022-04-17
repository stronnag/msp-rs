extern crate getopts;
use getopts::Options;
use std::env;

use std::convert::TryInto;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

mod msp;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] DEVICE", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let mut vers = 2;

    let mut opts = Options::new();
    opts.optopt("m", "mspvers", "set msp version", "2");
    opts.optflag("h", "help", "print this help menu");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            panic!("{}", f.to_string())
        }
    };

    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }

    let s = matches.opt_str("m");
    match s {
        Some(x) => vers = x.parse::<i32>().unwrap(),
        None => (),
    }

    let encode_msp_vers = |cmd, payload| {
        let vv = match vers {
            1 => msp::encode_msp(cmd, payload),
            _ => msp::encode_msp2(cmd, payload),
        };
        vv
    };

    let port_name = match matches.free.is_empty() {
        true => match serialport::available_ports() {
            Ok(ports) => match ports.len() {
                0 => {
                    println!("No serial ports found.");
                    return;
                }
                _ => ports[0].port_name.clone(),
            },
            Err(e) => {
                eprintln!("{:?}", e);
                return;
            }
        },
        false => matches.free[0].clone(),
    };

    println!("Serial port: {}", port_name);
    let mut port = serialport::new(port_name, 115_200)
        .timeout(Duration::from_millis(100))
        .open()
        .expect("Failed to open serial port");

    let mut writer = port.try_clone().expect("Failed to clone");
    let (tx, rx) = mpsc::channel();

    // reader
    let thr = thread::spawn(move || {
        msp::reader(&mut *port, tx.clone());
    });

    writer
        .write_all(&encode_msp_vers(msp::MSG_IDENT, &[]))
        .unwrap();

    for x in rx {
        match x.cmd {
            msp::MSG_IDENT => {
                if x.ok {
                    println!("MSP Vers: {}, (protocol v{})", x.data[0], vers);
                }
                writer
                    .write_all(&encode_msp_vers(msp::MSG_NAME, &[]))
                    .unwrap();
            }
            msp::MSG_NAME => {
                if x.ok {
                    println!("Name: {}", String::from_utf8_lossy(&x.data));
                }
                writer
                    .write_all(&encode_msp_vers(msp::MSG_API_VERSION, &[]))
                    .unwrap();
            }
            msp::MSG_API_VERSION => {
                if x.ok && x.len > 2 {
                    println!("API Version: {}.{}", x.data[1], x.data[2]);
                }
                writer
                    .write_all(&encode_msp_vers(msp::MSG_FC_VARIANT, &[]))
                    .unwrap();
            }
            msp::MSG_FC_VARIANT => {
                if x.ok {
                    println!("Firmware: {}", String::from_utf8_lossy(&x.data[0..4]));
                }
                writer
                    .write_all(&encode_msp_vers(msp::MSG_FC_VERSION, &[]))
                    .unwrap();
            }
            msp::MSG_FC_VERSION => {
                if x.ok {
                    println!("FW Version: {}.{}.{}", x.data[0], x.data[1], x.data[2]);
                }
                writer
                    .write_all(&encode_msp_vers(msp::MSG_BUILD_INFO, &[]))
                    .unwrap();
            }
            msp::MSG_BUILD_INFO => {
                if x.ok {
                    println!("Git revsion: {}", String::from_utf8_lossy(&x.data[19..]));
                }
                writer
                    .write_all(&encode_msp_vers(msp::MSG_BOARD_INFO, &[]))
                    .unwrap();
            }
            msp::MSG_BOARD_INFO => {
                if x.ok {
                    let board = if x.len > 8 {
                        String::from_utf8_lossy(&x.data[9..])
                    } else {
                        String::from_utf8_lossy(&x.data[0..4])
                    }
                    .to_string();
                    println!("Board: {}", board);
                }
                writer
                    .write_all(&encode_msp_vers(msp::MSG_WP_GETINFO, &[]))
                    .unwrap();
            }

            msp::MSG_WP_GETINFO => {
                if x.ok {
                    println!(
                        "Extant waypoints in FC: {} of {}, valid {}",
                        x.data[3],
                        x.data[1],
                        (x.data[2] == 1)
                    );
                }
                writer
                    .write_all(&encode_msp_vers(msp::MSG_ANALOG, &[]))
                    .unwrap();
            }
            msp::MSG_ANALOG => {
                if x.ok {
                    let volts: f32 = x.data[0] as f32 / 10.0;
                    println!("Voltage: {}", volts);
                }
                writer
                    .write_all(&encode_msp_vers(msp::MSG_RAW_GPS, &[]))
                    .unwrap();
            }
            msp::MSG_RAW_GPS => {
                // included as a more complex example
                if x.ok {
                    let fix = x.data[0];
                    let nsat = x.data[1];
                    let lat: f32 =
                        i32::from_le_bytes(x.data[2..6].try_into().unwrap()) as f32 / 1e7;
                    let lon: f32 =
                        i32::from_le_bytes(x.data[6..10].try_into().unwrap()) as f32 / 1e7;
                    let alt = i16::from_le_bytes(x.data[10..12].try_into().unwrap());
                    let spd: f32 =
                        u16::from_le_bytes(x.data[12..14].try_into().unwrap()) as f32 / 100.0;
                    let cog: f32 =
                        u16::from_le_bytes(x.data[14..16].try_into().unwrap()) as f32 / 10.0;
                    let hdop: f32 = if x.len > 16 {
                        u16::from_le_bytes(x.data[16..18].try_into().unwrap()) as f32 / 100.0
                    } else {
                        99.99
                    };
                    println!(
                        "GPS: fix {}, sats {}, lat, lon, alt {} {} {}, spd {} cog {} hdop {}",
                        fix, nsat, lat, lon, alt, spd, cog, hdop
                    );
                }
                return; // we're done
            }
            msp::MSG_DEBUGMSG => {
                if x.ok {
                    let s = String::from_utf8_lossy(&x.data);
                    println!("Debug: {}", s);
                }
            }
            _ => println!("Recv: {:#?}", x),
        }
    }
    thr.join().unwrap();
}
