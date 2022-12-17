extern crate crossbeam;
extern crate crossbeam_channel;
extern crate getopts;

use crossbeam_channel::{bounded, unbounded, Receiver, select};
use getopts::Options;
use std::convert::TryInto;
use std::env;
use std::time::Instant;
use std::time::Duration;
use std::thread;
use std::time;
use std::io::stdout;
use crossterm::{
    cursor::{Show, Hide, MoveToPreviousLine},
    queue,
    terminal::{Clear, ClearType},
    Result,
};
use std::io::Write;

mod msp;

fn ctrl_channel() -> std::result::Result<Receiver<()>, ctrlc::Error> {
    let (sender, receiver) = bounded(100);
    ctrlc::set_handler(move || {
        let _ = sender.send(());
    })?;
    Ok(receiver)
}

fn clean_exit() {
    queue!(stdout(),Show).unwrap();
    println!("\n\n\n");
    std::process::exit(0);
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] [device-node]", program);
    print!("{}", opts.usage(&brief));
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let mut vers = 2;
    let mut slow = false;
    let mut once = false;    
    let mut msgcnt = 0;
    
    let mut opts = Options::new();
    opts.optopt("m", "mspvers", "set msp version", "2");
    opts.optflag("1", "once", "exit after one iteration");
    opts.optflag("s", "slow", "slow mode");
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

    if matches.opt_present("s") {
        slow = true;
    }

    if matches.opt_present("1") {
        once = true;
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
        .timeout(Duration::from_millis(1200))
        .open()?;

    let ctrl_c_events = ctrl_channel().unwrap();
    
    let mut writer = port.try_clone()?;
    let (snd, rcv) = unbounded();
    queue!(stdout(),Hide).unwrap();
    
    crossbeam::scope(|s| {
        s.spawn(|_| {
            msp::reader(&mut *port, snd);
        });

        writer
            .write_all(&encode_msp_vers(msp::MSG_IDENT, &[]))
            .unwrap();

        let st = Instant::now();
        let mut upset = 3;
        loop {
            select! {
                recv(ctrl_c_events) -> _ => {
                    clean_exit();
                }

                recv(rcv) -> res => {
                    match res {
                        Ok(x) => {
                            let mut nxt = x.cmd;
                            msgcnt += 1;
                            match x.cmd {
                                msp::MSG_IDENT => {
                                    if x.ok {
                                        if x.len > 0 {
                                            println!("MSP Vers: {}, (protocol v{})", x.data[0], vers);
                                        }
                                    }
                                    nxt = msp::MSG_NAME
                                }
                                msp::MSG_NAME => {
                                    if x.ok {
                                        println!("Name: {}", String::from_utf8_lossy(&x.data));
                                    }
                                    nxt = msp::MSG_API_VERSION
                                }
                                msp::MSG_API_VERSION => {
                                    if x.ok && x.len > 2 {
                                        println!("API Version: {}.{}", x.data[1], x.data[2]);
                                    }
                                    nxt = msp::MSG_FC_VARIANT
                                }
                                msp::MSG_FC_VARIANT => {
                                    if x.ok {
                                        println!("Firmware: {}", String::from_utf8_lossy(&x.data[0..4]));
                                    }
                                    nxt = msp::MSG_FC_VERSION
                                }
                                msp::MSG_FC_VERSION => {
                                    if x.ok {
                                        println!("FW Version: {}.{}.{}", x.data[0], x.data[1], x.data[2]);
                                    }
                                    nxt = msp::MSG_BUILD_INFO
                                }
                                msp::MSG_BUILD_INFO => {
                                    if x.ok {
                                        if x.len > 19 {
                                            println!("Git revsion: {}", String::from_utf8_lossy(&x.data[19..]));
                                        }
                                    }
                                    nxt = msp::MSG_BOARD_INFO
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
                                    nxt = msp::MSG_WP_GETINFO
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
                                    nxt = if vers == 2 {
                                        msp::MSG_MISC2
                                    } else {
                                        msp::MSG_ANALOG
                                    };
                                }
                                msp::MSG_MISC2 => {
                                    if x.ok {
                                        let uptime = u32::from_le_bytes(x.data[0..4].try_into().unwrap());
                                        print!("Uptime: {}s", uptime);
                                        queue!(stdout(),  Clear(ClearType::UntilNewLine))?;
                                        println!();
                                        upset = 4;
                                    }
                                    nxt = msp::MSG_ANALOG 
                                }
                                
                                msp::MSG_ANALOG => {
                                    if x.ok {
                                        let volts: f32 = x.data[0] as f32 / 10.0;
                                        print!("Voltage: {:.2}", volts);
                                        queue!(stdout(),  Clear(ClearType::UntilNewLine))?;
                                        println!();
                                    }
                                    nxt = msp::MSG_RAW_GPS;
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
                                        print!(
                                            "GPS: fix {}, sats {}, {:.6}° {:.6}° {}m, spd {:.2} cog {:.0}", fix, nsat, lat, lon, alt, spd, cog);

                                        if x.len > 16 {
                                            let hdop: f32 = u16::from_le_bytes(x.data[16..18].try_into().unwrap()) as f32 / 100.0;
                                            print!(" hdop {:.2}", hdop);
                                        }

                                        queue!(stdout(),  Clear(ClearType::UntilNewLine))?;
                                        println!();
                                        let dura = st.elapsed();
                                        let duras: f64 = dura.as_secs() as f64 + dura.subsec_nanos() as f64 /1e9;
                                        let rate = msgcnt as f64 / duras;
                                        print!("Elapsed {:.2}s {} messages, rate {:.2}/s", duras, msgcnt, rate);
                                        queue!(stdout(),  Clear(ClearType::UntilNewLine))?;
                                        println!();
                                        queue!(stdout(),  MoveToPreviousLine(upset)).unwrap();
                                        stdout().flush()?;
                                    }

                                    if once {
                                        clean_exit();
                                    }
                                    if slow {
                                        thread::sleep(time::Duration::from_millis(1000));
                                    }
                                    nxt = if vers == 2 {
                                        msp::MSG_MISC2
                                    } else {
                                        msp::MSG_ANALOG
                                    };
                                }
                                msp::MSG_DEBUGMSG => {
                                    if x.ok {
                                        let s = String::from_utf8_lossy(&x.data);
                                        println!("Debug: {}", s);
                                        nxt = msp::MSG_IDENT
                                    }
                                }
                                _ => {
                                    println!("Recv: {:#?}", x);
                                    clean_exit();
                                },
                                       
                            }
                            writer.write_all(&encode_msp_vers(nxt, &[]))?;
                        },
                        Err(e) => panic!("{}",e),
                    }
                } 
            }
        } 
    }).unwrap() // crossbeam
}
