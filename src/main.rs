extern crate crossbeam_channel;
extern crate getopts;

use crossbeam_channel::{bounded, unbounded, Receiver, tick, select};
use getopts::Options;
use msp::MSPMsg;
use std::convert::TryInto;
use std::env;
use std::time::Instant;
use std::time::Duration;
use std::thread;
use std::time;
use crossterm::{
    cursor::*,
    execute,
    style::*,
    terminal::{size, Clear, ClearType, enable_raw_mode, disable_raw_mode},
    ExecutableCommand, Result,
    event::poll,
    event,
    event::{Event, KeyCode, KeyEvent},
};
use iota::iota;
use std::io::stdout;
use std::io;
mod msp;


iota! {
    const IY_PORT : u16 = 4 + iota;
    , IY_MW
        , IY_NAME
        , IY_APIV
        , IY_FC
        , IY_FCVERS
        , IY_BUILD
        , IY_BOARD
        , IY_WPINFO
        , IY_UPTIME
        , IY_ANALOG
        , IY_GPS
        , IY_ARM
        , IY_RATE
}

struct Prompt {
    y: u16,
    s: &'static str,
}


const UIPROMPTS: [Prompt; 14] = [
    Prompt {
        y: IY_PORT,
        s: "Port",
    },
    Prompt {
        y: IY_MW,
        s: "MW Vers",
    },
    Prompt {
        y: IY_NAME,
        s: "Name",
    },
    Prompt {
        y: IY_APIV,
        s: "API Vers",
    },
    Prompt { y: IY_FC, s: "FC" },
    Prompt {
        y: IY_FCVERS,
        s: "FC Vers",
    },
    Prompt {
        y: IY_BUILD,
        s: "Build",
    },
    Prompt {
        y: IY_BOARD,
        s: "Board",
    },
    Prompt {
        y: IY_WPINFO,
        s: "WP Info",
    },
    Prompt {
        y: IY_UPTIME,
        s: "Uptime",
    },

    Prompt {
        y: IY_ANALOG,
        s: "Power",
    },
    Prompt {
        y: IY_GPS,
        s: "GPS",
    },
    Prompt {
        y: IY_ARM,
        s: "Arming",
    },
    Prompt {
        y: IY_RATE,
        s: "Rate",
    },
];

fn outbase(y: u16, val: &str) -> Result<()> {
    stdout()
        .execute(MoveTo(0, y))?
        .execute(Print(val))?
        .execute(Clear(ClearType::UntilNewLine))?;
    Ok(())
}

fn outprompt(y: u16, val: &str) -> Result<()> {
    stdout()
        .execute(MoveTo(0, y))?
        .execute(Print(val))?
        .execute(MoveTo(8, y))?
        .execute(Print(":"))?
        .execute(MoveTo(10, y))?
        .execute(Print("---"))?
        .execute(Clear(ClearType::UntilNewLine))?;
    Ok(())
}

fn outvalue(y: u16, val: &str) -> Result<()> {
    stdout()
        .execute(MoveTo(10, y))?
        .execute(SetAttribute(Attribute::Bold))?
        .execute(Print(val))?
        .execute(SetAttribute(Attribute::Reset))?
        .execute(Clear(ClearType::UntilNewLine))?;
    Ok(())
}

fn outtitle(val: &str) -> Result<()> {
    stdout()
        .execute(MoveTo(30, 2))?
        .execute(SetAttribute(Attribute::Bold))?
        .execute(SetAttribute(Attribute::Reverse))?
        .execute(Print(val))?
        .execute(SetAttribute(Attribute::Reset))?
        .execute(Clear(ClearType::UntilNewLine))?;
    Ok(())
}

fn clean_exit(rows: u16) {
    disable_raw_mode().unwrap();
    outbase(rows-1,"").unwrap();
    execute!(stdout(), Show).unwrap();
    std::process::exit(0);
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] [device-node]", program);
    print!("{}", opts.usage(&brief));
}
fn get_serial_device(defdev: &str, testcvt: bool) -> String {
    let pname = match serialport::available_ports() {
        Ok(ports) => {
            for p in ports {
                match p.port_type {
                    serialport::SerialPortType::UsbPort(pt) => {
                        if (pt.vid == 0x0483 && pt.pid == 0x5740) ||
                            (pt.vid == 1659 && pt.pid == 8963) ||
                            (testcvt && (pt.vid == 0x10c4 && pt.pid == 0xea60)) {
                            return p.port_name.clone();
                        }
                    }
                    _ => (),
                }
            }
            defdev.to_string()
        },
        Err(_e) => {
            defdev.to_string()
        },
    };
    pname
}

fn ctrl_channel() -> std::result::Result<Receiver<()>, io::Error> {
    let (sender, receiver) = bounded(5);
    thread::spawn( move || {
        loop {
            if poll(Duration::ZERO).expect("") {
                if let Event::Key(event) = event::read().expect("Failed to read line") {
                    match event {
                        KeyEvent {
                            code: KeyCode::Char('q'),
                            modifiers: event::KeyModifiers::NONE, ..
                        } => {
                            let _ = sender.send(());
                            break
                        },
                        KeyEvent {
                            code: KeyCode::Char('c'),
                            modifiers: event::KeyModifiers::CONTROL, ..
                        } => {
                            let _ = sender.send(());
                            break
                        },
                        _ => {
                            thread::sleep(Duration::from_millis(50));
                        }
                    }
                };
            }
            thread::sleep(Duration::from_millis(50));
        }
    });
    Ok(receiver)
}


fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let mut vers = 2;
    let mut slow = false;
    let mut once = false;
    let mut msgcnt = 0;
    let mut timeout: u64 = 1000;
    let mut opts = Options::new();
    opts.optopt("m", "mspvers", "set msp version", "(2)");
    opts.optopt("t", "timeout", "set serial timeout (u/s)", "(1000)");
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

    match matches.opt_get::<u64>("t") {
        Ok(p) => match p {
            Some(px) => timeout = px,
            None => (),
        },
        Err(_) => (),
    }

    let s = matches.opt_str("m");
    match s {
        Some(x) => vers = x.parse::<u8>().unwrap(),
        None => (),
    }

    let defdev = if !matches.free.is_empty() {
        &matches.free[0]
    } else {
        "auto"
    };

    let encode_msp_vers = |cmd, payload| {
        let vv = match vers {
            1 => msp::encode_msp(cmd, payload),
            _ => msp::encode_msp2(cmd, payload),
        };
        vv
    };

    let ctrl_c_events = ctrl_channel().unwrap();

    let (_cols, rows) = size()?;

    enable_raw_mode()?;
    execute!(stdout(), Hide)?;
    execute!(stdout(), Clear(ClearType::All))?;


    'a:    loop {
        let pname: String;
        if defdev == "auto" {
            pname = get_serial_device(defdev, true);
        } else {
            pname = defdev.to_string();
        };

        let mut reader;

        outtitle("MSP Test Viewer")?;
        outbase(rows - 1, "Ctrl-C to exit")?;
        for i in 0..UIPROMPTS.len() {
            outprompt(UIPROMPTS[i].y, UIPROMPTS[i].s)?;
        }

        match serialport::new(&pname, 115_200)
            .timeout(Duration::from_micros(timeout))
            .open() {
                Ok(m) => {
                    reader = m;
                },
                Err(_) => {
                    let ticks = tick(Duration::from_millis(50));
                    let mut j = 0;
                    'c: loop {
                        select! {
                            recv(ticks) -> _ => {
                                j += 1;
                                if j == 20 {
                                    break 'c;
                                }
                            }
                            recv(ctrl_c_events) -> _ => {
                                break 'a;
                            }
                        }
                    }
                    continue 'a;
                }
            }
        let (snd, rcv) = unbounded();
        reader.clear(serialport::ClearBuffer::All)?;
        let mut writer = reader.try_clone()?;
        outvalue(IY_PORT, &pname)?;

        let thr  = thread::spawn(move || {
            msp::reader(&mut *reader, snd);
        });

        writer
            .write_all(&encode_msp_vers(msp::MSG_IDENT, &[]))
            .unwrap();

        let mut st = Instant::now();
        'b:     loop {
            select! {
                recv(ctrl_c_events) -> _ => {
                    clean_exit(rows);
                }

                recv(rcv) -> res => {
                    match res {
                        Ok(x) => {
                            if x.cmd  ==   msp::MSG_FAIL  {
                                thr.join().unwrap();
                                break 'b ();
                            }
                            msgcnt += 1;
                            let nxt = handle_msp(&mut st, x, msgcnt, vers, slow, once);
                            if nxt != msp::MSG_FAIL {
                                writer.write_all(&encode_msp_vers(nxt, &[]))?;
                            }
                        },
                        Err(e) => eprintln!("Recver {}",e),
                    }
                }
            }
        }
//        continue  'a;
    }
    clean_exit(rows);
    Ok(())
}


fn handle_msp( st: &mut std::time::Instant, x: MSPMsg, msgcnt: u64, vers: u8, slow: bool, once: bool) -> u16 {
    let mut nxt = x.cmd;
    match x.cmd {
        msp::MSG_IDENT => {
            *st = Instant::now();
            if x.ok {
                if x.len > 0 {
                    outvalue(IY_MW, &format!("MSP Vers: {}, (protocol v{})", x.data[0], vers)).unwrap();
                }
            }
            nxt = msp::MSG_NAME
        }
        msp::MSG_NAME => {
            if x.ok {
                outvalue(IY_NAME, &String::from_utf8_lossy(&x.data)).unwrap();
            }
            nxt = msp::MSG_API_VERSION
        }
        msp::MSG_API_VERSION => {
            if x.ok && x.len > 2 {
                outvalue(IY_APIV, &format!("API Version: {}.{}", x.data[1], x.data[2])).unwrap();
            }
            nxt = msp::MSG_FC_VARIANT
        }
        msp::MSG_FC_VARIANT => {
            if x.ok {
                outvalue(IY_FC, &String::from_utf8_lossy(&x.data[0..4])).unwrap();
            }
            nxt = msp::MSG_FC_VERSION
        }
        msp::MSG_FC_VERSION => {
            if x.ok {
                outvalue(IY_FCVERS, &format!("{}.{}.{}", x.data[0], x.data[1], x.data[2])).unwrap();
            }
            nxt = msp::MSG_BUILD_INFO
        }
        msp::MSG_BUILD_INFO => {
            if x.ok {
                if x.len > 19 {
                    outvalue(IY_BUILD, &String::from_utf8_lossy(&x.data[19..])).unwrap();
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
                };

                outvalue(IY_BOARD, &board).unwrap();
            }
            nxt = msp::MSG_WP_GETINFO
        }

        msp::MSG_WP_GETINFO => {
            if x.ok {
                outvalue(IY_WPINFO, &format!(
                    "Extant waypoints in FC: {} of {}, valid {}",
                    x.data[3],
                    x.data[1],
                    (x.data[2] == 1)
                )).unwrap();
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
                outvalue(IY_UPTIME, &format!("Uptime: {}s", uptime)).unwrap();
            }
            nxt = msp::MSG_ANALOG
        }

        msp::MSG_ANALOG => {
            if x.ok {
                let volts: f32 = x.data[0] as f32 / 10.0;
                outvalue(IY_ANALOG, &format!("{:.2} volts", volts)).unwrap();
            }
            if vers == 2 {
                nxt = msp::MSG_INAV_STATUS;
            } else {
                nxt = msp::MSG_STATUS_EX;
            }
        }

        msp::MSG_INAV_STATUS => {
            if x.ok {
                let armf = u32::from_le_bytes(x.data[9..13].try_into().unwrap());
                let s = get_armfails(armf);
                outvalue(IY_ARM, &s).unwrap();
                nxt = msp::MSG_RAW_GPS;
            } else {
                nxt = msp::MSG_STATUS_EX;
            }
        }

        msp::MSG_STATUS_EX => {
            let armf = u16::from_le_bytes(x.data[13..15].try_into().unwrap());
            let s = get_armfails(armf as u32);
            outvalue(IY_ARM, &s).unwrap();
            nxt = msp::MSG_RAW_GPS;
        }

        msp::MSG_RAW_GPS => {
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
                let mut s = format!(
                    "fix {}, sats {}, {:.6}° {:.6}° {}m, {:.0}m/s {:.0}°", fix, nsat, lat, lon, alt, spd, cog);
                if x.len > 16 {
                    let hdop: f32 = u16::from_le_bytes(x.data[16..18].try_into().unwrap()) as f32 / 100.0;
                    let s1 = format!(" hdop {:.2}", hdop);
                    s = s + &s1;
                }

                outvalue(IY_GPS, &s).unwrap();
                let dura = st.elapsed();
                let duras: f64 = dura.as_secs() as f64 + dura.subsec_nanos() as f64 /1e9;
                let rate = msgcnt as f64 / duras;
                outvalue(IY_RATE, &format!("Elapsed {:.2}s {} messages, rate {:.2}/s", duras, msgcnt, rate)).unwrap();
            }

            if once {
                nxt = msp::MSG_FAIL;
            } else {
                nxt = if vers == 2 {
                    msp::MSG_MISC2
                } else {
                    msp::MSG_ANALOG
                };
                if slow {
                    thread::sleep(time::Duration::from_millis(1000));
                }
            }
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
            nxt = msp::MSG_IDENT;
        },

    }
    return nxt;
}

fn get_armfails(reason: u32) -> String {
    const ARMFAILS: [&'static str; 32] = [
        "",
        "",
        "Armed",
        "",
        "",
        "",
        "",
        "F/S",
        "Level",
        "Calibrate",
        "Overload",
        "NavUnsafe",
        "MagCal",
        "AccCal",
        "ArmSwitch",
        "H/WFail",
        "BoxF/S",
        "BoxKill",
        "RCLink",
        "Throttle",
        "CLI",
        "CMS",
        "OSD",
        "Roll/Pitch",
        "Autotrim",
        "OOM",
        "Settings",
        "PWM Out",
        "PreArm",
        "DSHOTBeep",
        "Land",
        "Other",
    ];

    let s: String;
    if reason == 0 {
        s = "Ready to arm".to_string()
    } else {
        let mut v: Vec<String> = Vec::new();
        for i in 0..ARMFAILS.len() {
            if ((reason & (1 << i)) != 0) && ARMFAILS[i] != "" {
                v.push(ARMFAILS[i].to_string());
            }
        }
        s = v.join(" ");
    }
    return s;
}
