extern crate crossbeam;
extern crate crossbeam_channel;
extern crate getopts;

use crossbeam_channel::unbounded;
use getopts::Options;
use std::convert::TryInto;
use std::env;
use std::io;
use std::time::Duration;

mod msp;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] DEVICE", program);
    print!("{}", opts.usage(&brief));
}

fn main() -> io::Result<()> {
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
        return Ok(());
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
                    return Ok(());
                }
                _ => ports[0].port_name.clone(),
            },
            Err(_e) => {
                std::process::exit(1);
            }
        },
        false => matches.free[0].clone(),
    };

    println!("Serial port: {}", port_name);
    let mut port = serialport::new(port_name, 115_200)
        .timeout(Duration::from_millis(500))
        .open()?;

    let mut writer = port.try_clone()?;
    let (snd, rcv) = unbounded();

    crossbeam::scope(|s| {
        s.spawn(|_| {
            msp::reader(&mut *port, snd);
        });

        writer
            .write_all(&encode_msp_vers(msp::MSG_IDENT, &[]))
            .unwrap();

        loop {
            let x = rcv.recv().unwrap();
            let mut nxt = x.cmd;
            match x.cmd {
                msp::MSG_IDENT => {
                    if x.ok {
                        if x.len > 0 {
                            println!("MSP Vers: {}, (protocol v{})", x.data[0], vers);
                        }
                        nxt = msp::MSG_NAME
                    }
                }
                msp::MSG_NAME => {
                    if x.ok {
                        println!("Name: {}", String::from_utf8_lossy(&x.data));
                        nxt = msp::MSG_API_VERSION
                    }
                }
                msp::MSG_API_VERSION => {
                    if x.ok && x.len > 2 {
                        println!("API Version: {}.{}", x.data[1], x.data[2]);
                        nxt = msp::MSG_FC_VARIANT
                    }
                }
                msp::MSG_FC_VARIANT => {
                    if x.ok {
                        println!("Firmware: {}", String::from_utf8_lossy(&x.data[0..4]));
                        nxt = msp::MSG_FC_VERSION
                    }
                }
                msp::MSG_FC_VERSION => {
                    if x.ok {
                        println!("FW Version: {}.{}.{}", x.data[0], x.data[1], x.data[2]);
                        nxt = msp::MSG_BUILD_INFO
                    }
                }
                msp::MSG_BUILD_INFO => {
                    if x.ok {
                        println!("Git revsion: {}", String::from_utf8_lossy(&x.data[19..]));
                        nxt = msp::MSG_BOARD_INFO
                    }
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
                        nxt = msp::MSG_WP_GETINFO
                    }
                }

                msp::MSG_WP_GETINFO => {
                    if x.ok {
                        println!(
                            "Extant waypoints in FC: {} of {}, valid {}",
                            x.data[3],
                            x.data[1],
                            (x.data[2] == 1)
                        );
                        nxt = msp::MSG_ANALOG
                    }
                }
                msp::MSG_ANALOG => {
                    if x.ok {
                        let volts: f32 = x.data[0] as f32 / 10.0;
                        nxt = msp::MSG_RAW_GPS;
                        println!("Voltage: {:.2}", volts);
                    }
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
                            "GPS: fix {:2}, sats {:2}, lat, lon, alt {:.6} {:.6} {:2}, spd {:.2} cog {:5.1} hdop {:2}",
                            fix, nsat, lat, lon, alt, spd, cog, hdop
                        );
                        nxt = msp::MSG_ANALOG;
                    }
                }
                msp::MSG_DEBUGMSG => {
                    if x.ok {
                        let s = String::from_utf8_lossy(&x.data);
                        println!("Debug: {}", s);
                        nxt = msp::MSG_IDENT
                    }
                }
                _ => println!("Recv: {:#?}", x),
            }
            writer.write_all(&encode_msp_vers(nxt, &[]))?;
        }
    }).unwrap()
}
