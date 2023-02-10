extern crate crossbeam_channel;
extern crate getopts;
extern crate sys_info;

use crossbeam_channel::{bounded, select, tick, unbounded, Receiver};
use crossterm::{
    cursor::*,
    event,
    event::poll,
    event::{Event, KeyCode, KeyEvent},
    execute,
    style::*,
    terminal::{disable_raw_mode, enable_raw_mode, size, Clear, ClearType},
    ExecutableCommand, QueueableCommand, Result,
};
use getopts::Options;
use iota::iota;
use msp::MSPMsg;
use std::convert::TryInto;
use std::env;
use std::io;
use std::io::stdout;
use std::io::*;
use std::thread;
use std::time;
use std::time::Duration;
use std::time::Instant;
use sys_info::*;

mod msp;

#[cfg_attr(unix, path = "serial_posix.rs")]
#[cfg_attr(windows, path = "serial_windows.rs")]
mod serial;

use crate::serial::SerialDevice;

const VERSION: &str = env!("CARGO_PKG_VERSION");

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

#[cfg(target_os = "linux")]
fn get_ostype() -> String {
    let lx = linux_os_release().unwrap();
    lx.name().to_string()
}

#[cfg(not(target_os = "linux"))]
fn get_ostype() -> String {
    os_type().unwrap_or("unknown".to_string())
}

fn get_rel_info() -> String {
    let osrel = os_release().unwrap_or("unknown".to_string());
    let ostype = get_ostype();
    format!(
        "v{} on {} {} {}",
        VERSION,
        &ostype,
        &osrel,
        std::env::consts::ARCH
    )
}

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

fn outtitle(val: &str, cols: u16) -> Result<()> {
    setcentre(val, cols, 1)?;
    stdout()
        .execute(SetAttribute(Attribute::Bold))?
        .execute(SetAttribute(Attribute::Reverse))?
        .execute(Print(val))?
        .execute(SetAttribute(Attribute::Reset))?
        .execute(Clear(ClearType::UntilNewLine))?;
    Ok(())
}

fn outsubtitle(val: &str, cols: u16) -> Result<()> {
    setcentre(val, cols, 2)?;
    stdout()
        .execute(Print(val))?
        .execute(Clear(ClearType::UntilNewLine))?;
    Ok(())
}

fn setcentre(val: &str, cols: u16, row: u16) -> Result<()> {
    let n = val.len() as u16;
    let xp = (cols - n) / 2_u16;
    stdout().queue(MoveTo(xp, row))?;
    Ok(())
}

fn redraw(cols: u16, rows: u16) -> Result<()> {
    outtitle("MSP Test Viewer", cols)?;
    outbase(rows - 1, "Ctrl-C to exit")?;
    for e in &UIPROMPTS {
        outprompt(e.y, e.s)?;
    }
    outsubtitle(&get_rel_info(), cols)?;
    Ok(())
}

fn clean_exit(rows: u16) {
    disable_raw_mode().unwrap();
    outbase(rows - 1, "").unwrap();
    execute!(stdout(), Show).unwrap();
    std::process::exit(0);
}

fn print_usage(program: &str, opts: &Options) {
    let brief = format!(
        "Usage: {} [options] [device-node]\nVersion: {}",
        program, VERSION
    );
    print!("{}", opts.usage(&brief));
}

fn ctrl_channel() -> std::result::Result<Receiver<u8>, io::Error> {
    let (sender, receiver) = bounded(5);
    thread::spawn(move || loop {
        if poll(Duration::ZERO).expect("") {
            if let Event::Key(event) = event::read().expect("Failed to read line") {
                match event {
                    KeyEvent {
                        code: KeyCode::Char('q'),
                        modifiers: event::KeyModifiers::NONE,
                        ..
                    } => {
                        let _ = sender.send(b'Q');
                        break;
                    }
                    KeyEvent {
                        code: KeyCode::Char('r'),
                        modifiers: event::KeyModifiers::NONE,
                        ..
                    } => {
                        let _ = sender.send(b'R');
                    }
                    KeyEvent {
                        code: KeyCode::Char('c'),
                        modifiers: event::KeyModifiers::CONTROL,
                        ..
                    } => {
                        let _ = sender.send(b'Q');
                        break;
                    }
                    _ => {
                        thread::sleep(Duration::from_millis(50));
                    }
                }
            };
        }
        thread::sleep(Duration::from_millis(50));
    });
    Ok(receiver)
}

pub fn get_serial_device(defdev: &str, testcvt: bool) -> String {
    match serialport::available_ports() {
        Ok(ports) => {
            for p in ports {
                match &p.port_type {
                    serialport::SerialPortType::UsbPort(pt) => {
                        if (pt.vid == 0x0483 && pt.pid == 0x5740)
                            || (pt.vid == 0x0403 && pt.pid == 0x6001)
                            || (testcvt && (pt.vid == 0x10c4 && pt.pid == 0xea60))
                        {
                            return p.port_name.clone();
                        }
                    }
                    _ => {
                        if std::env::consts::OS == "freebsd" && &p.port_name[0..9] == "/dev/cuaU" {
                            return p.port_name.clone();
                        }
                    }
                }
            }
            defdev.to_string()
        }
        Err(_e) => defdev.to_string(),
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let mut vers = 1;
    let mut slow = false;
    let mut once = false;
    let mut opts = Options::new();
    opts.optflag("s", "slow", "slow mode");
    opts.optflag("1", "once", "Single iteration, then exit");
    opts.optflag("v", "version", "Show version");
    opts.optflag("h", "help", "print this help menu");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            panic!("{}", f.to_string())
        }
    };

    if matches.opt_present("h") {
        print_usage(&program, &opts);
        return Ok(());
    }

    if matches.opt_present("v") {
        println!("{}", VERSION);
        return Ok(());
    }

    if matches.opt_present("s") {
        slow = true;
    }

    if matches.opt_present("s") {
        slow = true;
    }

    if matches.opt_present("1") {
        once = true;
    }

    let defdev = if !matches.free.is_empty() {
        &matches.free[0]
    } else {
        "auto"
    };

    let encode_msp_vers = |cmd, payload, version| match version {
        1 => msp::encode_msp(cmd, payload),
        _ => msp::encode_msp2(cmd, payload),
    };

    let ctrl_c_events = ctrl_channel().unwrap();

    let (mut cols, mut rows) = size()?;

    enable_raw_mode()?;
    execute!(stdout(), Hide)?;
    execute!(stdout(), Clear(ClearType::All))?;

    let mut sd = serial::SerialDevice::new();
    'a: loop {
        let pname: String = if defdev == "auto" {
            get_serial_device(defdev, true)
        } else {
            defdev.to_string()
        };

        redraw(cols, rows)?;

        match sd.open(&pname, 115_200) {
            Ok(_) => sd.clear(),
            Err(_e) => {
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
                    recv(ctrl_c_events) -> res => {
                        if let Ok(x) = res  {
                        if x == b'Q' { break 'a;}
                        }
                    }
                            }
                }
                continue 'a;
            }
        }

        let (snd, rcv) = unbounded();

        outvalue(IY_PORT, &pname)?;

        let rd = sd.clone();
        let thr = thread::spawn(move || {
            msp::reader(rd, snd);
        });

        let mut nto = 0;
        sd.write_all(&msp::encode_msp(msp::MSG_IDENT, &[]))?;
        let ticks = tick(Duration::from_millis(100));
        let mut st = Instant::now();
        let mut mtimer = Instant::now();
        let mut refresh = false;
        let mut msgcnt = 0;
        let mut e_bad = 0;
        let mut e_crc = 0;

        'b: loop {
            select! {
                    recv(ticks) -> _ => {
                        if mtimer.elapsed() > Duration::from_millis(5000) {
                            vers  = 1;
                            nto += 1;
                            outvalue(IY_RATE, &format!("Timeout ({})", nto))?;
                            mtimer = Instant::now();
                            sd.write_all(&msp::encode_msp(msp::MSG_IDENT, &[]))?;
                        }

                if msgcnt > 0 {
                let dura = st.elapsed();
                let duras: f64 = dura.as_secs() as f64 + dura.subsec_nanos() as f64 / 1e9;
                let rate = msgcnt as f64 / duras;
                outvalue(
                    IY_RATE,
                    &format!("{} messages in {:.1}s ({:.1}/s) (unknown: {}, crc {})", msgcnt, duras, rate, e_bad, e_crc))?;
                }
            }

                    recv(ctrl_c_events) -> res => {
                if let Ok(x) = res {
                if x == b'Q' { clean_exit(rows);}
                refresh = true;
                }
                    }

                    recv(rcv) -> res => {
                        let mut nxt: u16;
                        mtimer = Instant::now();
                        match res {
                            Ok(x) => {
                    if msgcnt == 0 {
                    st = Instant::now();
                    e_crc = 0;
                    e_bad = 0;
                    }
                                msgcnt += 1;
                    let _last = x.cmd;
                                match x.ok {
                                    msp::MSPRes::Ok => {
                                        if let Some(i) = handle_msp(x, &mut vers, slow, once) {
                                            nxt = i;
                                        } else {
                                            break 'a;
                                        }
                                    },
                                    msp::MSPRes::Crc => {
                        e_crc += 1;
                                        nxt = msp::MSG_IDENT;
                                    },
                                    msp::MSPRes::Dirn => {
                        e_bad += 1;
                                        nxt = match x.cmd {
                                            msp::MSG_IDENT => msp::MSG_NAME,
                                            msp::MSG_NAME => msp::MSG_API_VERSION,
                                            msp::MSG_API_VERSION => msp::MSG_FC_VARIANT,
                                            msp::MSG_FC_VARIANT => msp::MSG_FC_VERSION,
                                            msp::MSG_FC_VERSION => msp::MSG_BUILD_INFO,
                                            msp::MSG_BUILD_INFO => msp::MSG_BOARD_INFO,
                                            msp::MSG_BOARD_INFO => {
                                                 outvalue(IY_BOARD, "MultiWii")?;
                                                msp::MSG_WP_GETINFO
                                            },
                                            msp::MSG_WP_GETINFO => msp::MSG_ANALOG,
                                            msp::MSG_MISC2 => msp::MSG_ANALOG,
                                            msp::MSG_INAV_STATUS => msp::MSG_STATUS_EX,
                                            msp::MSG_STATUS_EX =>  msp::MSG_RAW_GPS,
                                            _ => msp::MSG_IDENT,
                                        };
                                    },
                                    msp::MSPRes::Fail => {
                                        thr.join().unwrap();
                        break 'b ;
                                    },
                                }
                    if nxt == msp::MSG_IDENT  {
                    vers = 1;
                    msgcnt = 0;
                    }
                    if refresh {
                    refresh  = false;
                    nxt = msp::MSG_IDENT;
                    (cols, rows) = size()?;
                    execute!(stdout(), Clear(ClearType::All))?;
                    redraw(cols, rows)?;
                    outvalue(IY_PORT, &pname)?;
                    }
                    _ = sd.write(&encode_msp_vers(nxt, &[], vers));
                            },
                            Err(e) => eprintln!("Recv-err {}",e)
                        }
                    }
                }
        }
    }
    clean_exit(rows);
    Ok(())
}

fn handle_msp(x: MSPMsg, vers: &mut u8, slow: bool, once: bool) -> Option<u16> {
    let nxt: Option<u16>;
    match x.cmd {
        msp::MSG_IDENT => {
            if x.len > 0 {
                outvalue(IY_MW, &format!("MSP Vers: {}, (MSP v{})", x.data[0], *vers)).unwrap();
            }
            nxt = Some(msp::MSG_NAME)
        }
        msp::MSG_NAME => {
            outvalue(IY_NAME, &String::from_utf8_lossy(&x.data)).unwrap();
            nxt = Some(msp::MSG_API_VERSION)
        }
        msp::MSG_API_VERSION => {
            if x.len > 2 {
                if x.data[1] > 1 && x.data[2] > 0 && *vers == 1 {
                    *vers = 2;
                }
                outvalue(
                    IY_APIV,
                    &format!("{}.{} (MSP v{})", x.data[1], x.data[2], *vers),
                )
                .unwrap();
            }
            nxt = Some(msp::MSG_FC_VARIANT)
        }
        msp::MSG_FC_VARIANT => {
            outvalue(IY_FC, &String::from_utf8_lossy(&x.data[0..4])).unwrap();
            nxt = Some(msp::MSG_FC_VERSION)
        }
        msp::MSG_FC_VERSION => {
            outvalue(
                IY_FCVERS,
                &format!("{}.{}.{}", x.data[0], x.data[1], x.data[2]),
            )
            .unwrap();
            nxt = Some(msp::MSG_BUILD_INFO)
        }
        msp::MSG_BUILD_INFO => {
            if x.len > 19 {
                let txt = format!(
                    "{} {} ({})",
                    &String::from_utf8_lossy(&x.data[0..11]),
                    &String::from_utf8_lossy(&x.data[11..19]),
                    &String::from_utf8_lossy(&x.data[19..])
                );
                outvalue(IY_BUILD, &txt).unwrap();
            }
            nxt = Some(msp::MSG_BOARD_INFO)
        }
        msp::MSG_BOARD_INFO => {
            let board = if x.len > 8 {
                String::from_utf8_lossy(&x.data[9..])
            } else {
                String::from_utf8_lossy(&x.data[0..4])
            };
            outvalue(IY_BOARD, &board).unwrap();
            nxt = Some(msp::MSG_WP_GETINFO)
        }

        msp::MSG_WP_GETINFO => {
            outvalue(
                IY_WPINFO,
                &format!("{} of {}, valid {}", x.data[3], x.data[1], (x.data[2] == 1)),
            )
            .unwrap();
            nxt = if *vers == 2 {
                Some(msp::MSG_MISC2)
            } else {
                Some(msp::MSG_ANALOG)
            };
        }
        msp::MSG_MISC2 => {
            let uptime = u32::from_le_bytes(x.data[0..4].try_into().unwrap());
            outvalue(IY_UPTIME, &format!("{}s", uptime)).unwrap();
            nxt = Some(msp::MSG_ANALOG)
        }

        msp::MSG_ANALOG => {
            let volts: f32 = x.data[0] as f32 / 10.0;
            let amps: f32 = u16::from_le_bytes(x.data[5..7].try_into().unwrap()) as f32 / 100.0;
            outvalue(IY_ANALOG, &format!("{:.1} volts, {:2} amps", volts, amps)).unwrap();
            nxt = if *vers == 2 {
                Some(msp::MSG_INAV_STATUS)
            } else {
                Some(msp::MSG_STATUS_EX)
            };
        }

        msp::MSG_INAV_STATUS => {
            let armf = u32::from_le_bytes(x.data[9..13].try_into().unwrap());
            let s = get_armfails(armf);
            outvalue(IY_ARM, &s).unwrap();
            nxt = Some(msp::MSG_RAW_GPS);
        }

        msp::MSG_STATUS_EX => {
            let armf = u16::from_le_bytes(x.data[13..15].try_into().unwrap());
            let s = get_armfails(armf as u32);
            outvalue(IY_ARM, &s).unwrap();
            nxt = Some(msp::MSG_RAW_GPS);
        }

        msp::MSG_RAW_GPS => {
            let fix = x.data[0];
            let nsat = x.data[1];
            let lat: f32 = i32::from_le_bytes(x.data[2..6].try_into().unwrap()) as f32 / 1e7;
            let lon: f32 = i32::from_le_bytes(x.data[6..10].try_into().unwrap()) as f32 / 1e7;
            let alt = i16::from_le_bytes(x.data[10..12].try_into().unwrap());
            let spd: f32 = u16::from_le_bytes(x.data[12..14].try_into().unwrap()) as f32 / 100.0;
            let cog: f32 = u16::from_le_bytes(x.data[14..16].try_into().unwrap()) as f32 / 10.0;
            let mut s = format!(
                "fix {}, sats {}, {:.6}° {:.6}° {}m, {:.0}m/s {:.0}°",
                fix, nsat, lat, lon, alt, spd, cog
            );
            if x.len > 16 {
                let hdop: f32 =
                    u16::from_le_bytes(x.data[16..18].try_into().unwrap()) as f32 / 100.0;
                let s1 = format!(" hdop {:.2}", hdop);
                s = s + &s1;
            }

            outvalue(IY_GPS, &s).unwrap();
            nxt = if once {
                None
            } else if *vers == 2 {
                Some(msp::MSG_MISC2)
            } else {
                Some(msp::MSG_ANALOG)
            };
            if slow {
                thread::sleep(time::Duration::from_millis(1000));
            }
        }
        msp::MSG_DEBUGMSG => {
            let s = String::from_utf8_lossy(&x.data);
            println!("Debug: {}", s);
            nxt = Some(msp::MSG_IDENT)
        }
        _ => {
            println!("Recv: {:#?}", x);
            nxt = Some(msp::MSG_IDENT)
        }
    }
    nxt
}

fn get_armfails(reason: u32) -> String {
    const ARMFAILS: [&str; 32] = [
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

    let s: String = if reason == 0 {
        "Ready to arm".to_string()
    } else {
        let mut v: Vec<String> = Vec::new();
        for (i, e) in ARMFAILS.iter().enumerate() {
            if ((reason & (1 << i)) != 0) && !e.is_empty() {
                v.push(e.to_string());
            }
        }
        v.push(format!("(0x{:x})", reason));
        v.join(" ")
    };
    s
}
