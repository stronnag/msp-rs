
use libc::{c_int, c_char, size_t, ssize_t};
use std::ffi::CString;

#[link(name = "serial")]
extern {
    fn open_serial(name: *const c_char, baud: c_int) -> c_int;
    fn read_serial(fd: c_int , buf: *mut u8, buflen: size_t) -> ssize_t;
    fn write_serial(fd: c_int , buf: *const u8, buflen: size_t) -> ssize_t;
    fn close_serial(fd: c_int);
    fn flush_serial(fd: c_int);
}

pub fn open(name: &str, baud: isize) -> Option<isize> {
    let dname = CString::new(name).unwrap();
    let dptr = dname.as_ptr() as *const c_char;
    unsafe {
    let fd = open_serial(dptr, baud as c_int);
	if fd < 0 {
	    None
	} else {
	    Some(fd as isize)
	}
    }
}

pub fn read(fd: isize, size: usize) -> Option<Vec<u8>> {
    unsafe {
	let mut dst = Vec::with_capacity(size);
	let psrc = dst.as_mut_ptr();
	let n = read_serial(fd as c_int, psrc, size as size_t);
	if n > 0 {
	    dst.set_len(n as usize);
	    Some(dst)
	} else {
	    None
	}
    }
}

pub fn write(fd: isize, src: &[u8]) -> isize {
    unsafe {
	let n = write_serial(fd as c_int, src.as_ptr(), src.len());
	n as isize
    }
}

pub fn close (fd: isize) {
    unsafe { close_serial(fd as c_int)}
}

pub fn flush (fd: isize) {
    unsafe { flush_serial(fd as c_int)}
}
