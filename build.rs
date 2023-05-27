extern crate cc;

fn main() {
    println!("cargo:rerun-if-changed=src/serial.c");
    cc::Build::new().file("src/serial.c").compile("libserial.a");
}
