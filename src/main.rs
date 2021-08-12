extern crate getopts;
use getopts::Options;
use std::env;

use std::time::Duration;
use std::thread;
use std::sync::mpsc;

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
        Ok(m) => { m }
        Err(f) => { panic!("{}", f.to_string()) }
    };

    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }

    let s = matches.opt_str("m");
    match s {
        Some(x) => vers = x.parse::<i32>().unwrap(),
        None => ()
    }

    let encode_msp_vers = |cmd, payload| {
	let vv = match vers {
	    1 => msp::encode_msp(cmd, payload),
	    _ => msp::encode_msp2(cmd, payload),
	};
	vv
    };

    let port_name = match matches.free.is_empty() {
	true => serialport::available_ports().expect("No serial port")[0].port_name.clone(),
	false => matches.free[0].clone(),
    };

    println!("Serial port: {}", port_name);
    let mut port = serialport::new(port_name, 115_200)
	.timeout(Duration::from_millis(100))
        .open().expect("Failed to open serial port");

    let mut clone = port.try_clone().expect("Failed to clone");
    let (tx,  rx) = mpsc::channel();

    // reader
    let thr = thread::spawn(move || {
	msp::reader(&mut *port, tx.clone());
    });

    let mut vv = encode_msp_vers(msp::MSG_IDENT, &[]);

    clone.write_all(&vv).unwrap();

    for x in rx {
	match x.cmd {
	    msp::MSG_IDENT => {
		println!("MSP Vers: {}, (protocol v{})", x.data[0], vers);
		vv = encode_msp_vers(msp::MSG_NAME, &[]);
		clone.write_all(&vv).unwrap();
	    },
	    msp::MSG_NAME => {
		let s = String::from_utf8_lossy(&x.data);
		println!("Name: {}", s);
	    },
	    _ => println!("Recv: {:#?}", x)
	}
    }
    thr.join().unwrap();
}
