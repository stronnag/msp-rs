use serialport::SerialPort;
use std::io;
//use std::io::Write; // <--- bring flush() into scope

pub const MSG_IDENT: u16 = 100;
pub const MSG_NAME: u16 = 10;
pub const MSG_API_VERSION: u16 = 1;
pub const MSG_FC_VARIANT: u16 = 2;
pub const MSG_FC_VERSION: u16 = 3;
pub const MSG_BOARD_INFO: u16 = 4;
pub const MSG_BUILD_INFO: u16 = 5;
pub const MSG_WP_GETINFO: u16 = 20;
pub const MSG_RAW_GPS: u16 = 106;
pub const MSG_ANALOG: u16 = 110;
pub const MSG_DEBUGMSG: u16 = 253;
pub const MSG_MISC2: u16 = 0x203a;

fn crc8_dvb_s2(mut c: u8, a: u8) -> u8 {
    c ^= a;
    for _ in 0..8 {
        if (c & 0x80) != 0 {
            c = (c << 1) ^ 0xd5
        } else {
            c = c << 1
        }
    }
    c
}

pub fn encode_msp2(cmd: u16, payload: &[u8]) -> Vec<u8> {
    let mut paylen = 0u16;
    let payl = payload.len();
    if payl > 0 {
        paylen = payl as u16;
    }
    let mut v: Vec<u8> = Vec::new();
    v.push(b'$');
    v.push(b'X');
    v.push(b'<');
    v.push(0);
    v.push((cmd & 0xff) as u8);
    v.push((cmd >> 8) as u8);
    v.push((paylen & 0xff) as u8);
    v.push((paylen >> 8) as u8);

    for x in payload.iter() {
        v.push(*x);
    }
    let mut crc: u8 = 0;
    for i in 3..payl + 8 {
        crc = crc8_dvb_s2(crc, v[i]);
    }
    v.push(crc);
    v
}

pub fn encode_msp(cmd: u16, payload: &[u8]) -> Vec<u8> {
    let mut paylen = 0u8;
    let payl = payload.len();
    if payl > 0 {
        paylen = payl as u8;
    }
    let mut v: Vec<u8> = Vec::new();
    v.push(b'$');
    v.push(b'M');
    v.push(b'<');
    v.push(paylen);
    v.push(cmd as u8);
    for x in payload.iter() {
        v.push(*x);
    }
    let mut crc: u8 = 0;
    for i in 3..payl + 5 {
        crc ^= v[i];
    }
    v.push(crc);
    v
}

enum States {
    Init,
    M,
    Dirn,
    Len,
    Cmd,
    Data,
    Crc,

    XHeader2,
    XFlags,
    XId1,
    XId2,
    XLen1,
    XLen2,
    XData,
    XChecksum,
}

#[derive(Debug, Default, Clone)]
pub struct MSPMsg {
    pub len: u16,
    pub cmd: u16,
    pub ok: bool,
    pub data: Vec<u8>,
}

/*
fn timeout_marker(val: u32) {
    let idx = val % 4;
    let c: char;
    match idx {
        0 => c = '|',
        1 => c = '/',
        2 => c = '-',
        3 => c = '\\',
        4_u32..=u32::MAX => todo!(),
    }
    print!("{}\x08", c);
    io::stdout().flush().unwrap();
}
*/
pub fn reader(port: &mut dyn SerialPort, tx: crossbeam::channel::Sender<MSPMsg>) {
    let mut msg = MSPMsg::default();
    let mut n = States::Init;
    let mut inp: [u8; 256] = [0; 256];
    let mut crc = 0u8;
    let mut count = 0u16;
//    let mut tcount = 0u32;

    loop {
        match port.read(inp.as_mut_slice()) {
            Ok(t) => {
                for j in 0..t {
                    match n {
                        States::Init => {
                            if inp[j] == b'$' {
                                n = States::M;
                                msg.ok = false;
                                msg.len = 0;
                                msg.cmd = 0;
                            }
                        }
                        States::M => {
                            n = match inp[j] {
                                b'M' => States::Dirn,
                                b'X' => States::XHeader2,
                                _ => States::Init,
                            }
                        }
                        States::Dirn => match inp[j] {
                            b'!' => n = States::Len,
                            b'>' => {
                                n = States::Len;
                            }
                            _ => n = States::Init,
                        },
                        States::XHeader2 => match inp[j] {
                            b'!' => n = States::XFlags,
                            b'>' => {
                                n = States::XFlags;
                            }
                            _ => n = States::Init,
                        },
                        States::XFlags => {
                            crc = crc8_dvb_s2(0, inp[j]);
                            n = States::XId1;
                        }
                        States::XId1 => {
                            crc = crc8_dvb_s2(crc, inp[j]);
                            msg.cmd = inp[j] as u16;
                            n = States::XId2;
                        }
                        States::XId2 => {
                            crc = crc8_dvb_s2(crc, inp[j]);
                            msg.cmd |= (inp[j] as u16) << 8;
                            n = States::XLen1;
                        }
                        States::XLen1 => {
                            crc = crc8_dvb_s2(crc, inp[j]);
                            msg.len = inp[j] as u16;
                            n = States::XLen2;
                        }
                        States::XLen2 => {
                            crc = crc8_dvb_s2(crc, inp[j]);
                            msg.len |= (inp[j] as u16) << 8;
                            if msg.len > 0 {
                                n = States::XData;
                                count = 0;
                                msg.data = vec![0; msg.len.into()];
                            } else {
                                n = States::XChecksum;
                            }
                        }
                        States::XData => {
                            crc = crc8_dvb_s2(crc, inp[j]);
                            msg.data[count as usize] = inp[j];
                            count += 1;
                            if count == msg.len {
                                n = States::XChecksum;
                            }
                        }
                        States::XChecksum => {
                            if crc != inp[j] {
                                println!(
                                    "XCRC error on {} {} {} l={}",
                                    msg.cmd, crc, inp[j], msg.len
                                );
                                msg.ok = false
                            } else {
                                msg.ok = true
                            }
                            tx.send(msg.clone()).unwrap();
                            n = States::Init;
                        }
                        States::Len => {
                            msg.len = inp[j] as u16;
                            crc = inp[j];
                            n = States::Cmd;
                        }
                        States::Cmd => {
                            msg.cmd = inp[j] as u16;
                            crc ^= inp[j];
                            if msg.len == 0 {
                                n = States::Crc;
                            } else {
                                msg.data = vec![0; msg.len.into()];
                                n = States::Data;
                                count = 0;
                            }
                        }
                        States::Data => {
                            msg.data[count as usize] = inp[j];
                            crc ^= inp[j];
                            count += 1;
                            if count == msg.len {
                                n = States::Crc;
                            }
                        }
                        States::Crc => {
                            if crc != inp[j] {
                                println!("MCRC error on {} {} {}", msg.cmd, crc, inp[j]);
                                msg.ok = false;
                            } else {
                                msg.ok = true
                            }
                            tx.send(msg.clone()).unwrap();
                            n = States::Init;
                        }
                    }
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::BrokenPipe => return,
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
//                timeout_marker(tcount);
//                tcount += 1;
            }
            Err(e) => {
                println!("{:?}", e);
                msg.len = 0;
                msg.ok = false;
                tx.send(msg.clone()).unwrap();
                n = States::Init
            }
        }
    }
}
