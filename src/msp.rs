use serialport::SerialPort;
use std::sync::mpsc;
use std::io;

fn crc8_dvb_s2(mut c: u8, a: u8) -> u8 {
    c ^= a;
    for _ in 0..8 {
        if (c & 0x80) != 0 {
            c = (c << 1) ^ 0xd5
        } else {
            c = c << 1
        }
    }
    return c
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
    for i in 3..payl+8 {
	crc = crc8_dvb_s2(crc, v[i]);
    }
    v.push(crc);
    return v;
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
    for i in 3..payl+5 {
	crc ^= v[i];
    }
    v.push(crc);
    return v;
}

pub const MSG_IDENT: u16 = 100;
pub const MSG_NAME: u16 = 10;

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
    XChecksum
}

#[derive(Debug, Default, Clone)]
pub struct MSPMsg {
    pub len:  u16,
    pub cmd:  u16,
    pub ok:   bool,
    pub data: Vec<u8>
}

pub fn reader<T: SerialPort + ?Sized >(port: &mut T, tx: mpsc::Sender<MSPMsg>) {
    let mut msg = MSPMsg::default();
    let mut n = States::Init;
    let mut inp: [u8; 128] = [0; 128];
    let mut crc = 0u8;
    let mut count  = 0u16;
    loop {
	match port.read(&mut inp) {
	    Ok(_) => {
                for j in inp.iter() { // .iter() and *j required for older rustc, alas
		    match n {
			States::Init => {
			    if *j == b'$' {
                                n = States::M;
                                msg.ok = false;
                                msg.len = 0;
                                msg.cmd = 0;
			    }
			},
			States::M => {
			    n = match *j {
				b'M' =>  States::Dirn,
				b'X' =>  States::XHeader2,
				_ =>  States::Init
			    }
			},
			States::Dirn => {
			    match *j {
				b'!' => n = States::Len,
				b'>' => { n = States::Len; msg.ok = true },
				_ => n = States::Init
			    }
			},
			States::XHeader2 => {
			    match *j {
				b'!' => n = States::XFlags,
				b'>' => { n = States::XFlags; msg.ok = true },
				_ => n = States::Init
			    }
			},
			States::XFlags => {
                            crc = crc8_dvb_s2(0, *j);
                            n = States::XId1;
			},
                        States::XId1 => {
                            crc = crc8_dvb_s2(crc, *j);
                            msg.cmd = *j as u16;
			    n = States::XId2;

			},
                        States::XId2 => {
                            crc = crc8_dvb_s2(crc, *j);
                            msg.cmd |= (*j as u16) << 8;
			    n = States::XLen1;
			},
                        States::XLen1 => {
                            crc = crc8_dvb_s2(crc, *j);
                            msg.len = *j as u16;
			    n = States::XLen2;
			},
                        States::XLen2 => {
                            crc = crc8_dvb_s2(crc, *j);
                            msg.len |= (*j as u16) << 8;
			    if msg.len > 0 {
				n = States::XData;
                                count = 0;
                                msg.data = vec![0; msg.len.into()];
			    } else {
				n = States::XChecksum;
			    }
			},
			States::XData => {
                            crc = crc8_dvb_s2(crc, *j);
                            msg.data[count as usize] = *j;
                            count += 1;
                            if count == msg.len {
                                n = States::XChecksum;
                            }
			},
                        States::XChecksum => {
                            if crc != *j {
                                println!("CRC error on {}", msg.cmd)
                            } else {
				tx.send(msg.clone()).unwrap();
                            }
                            n = States::Init;
			},
			States::Len => {
                            msg.len = *j as u16;
                            crc = *j;
                            n = States::Cmd;
			},
			States::Cmd => {
                            msg.cmd = *j as u16;
                            crc ^= *j;
                            if msg.len == 0 {
                                n = States::Crc;
                            } else {
                                msg.data = vec![0; msg.len.into()];
                                n = States::Data;
                                count = 0;
                            }
			},
                        States::Data => {
                            msg.data[count as usize] = *j;
                            crc ^= *j;
                            count += 1;
                            if count == msg.len {
                                n = States::Crc;
                            }
			},
                        States::Crc => {
			    if crc != *j {
                                println!("CRC error on {}", msg.cmd)
                            } else {
				tx.send(msg.clone()).unwrap();
                            }
                            n = States::Init;
			}
		    }
                }
	    }
	    Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
	    Err(e) => {
		eprintln!("{:?}", e);
		return;
	    },
        }
    }
}
