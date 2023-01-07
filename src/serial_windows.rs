use std::io;
use std::io::Error;
use libc::{c_int, c_char, size_t, ssize_t};

use winapi::um::handleapi::*;
use winapi::um::winnt::HANDLE;
use std::ffi::CString;

#[link(name = "serial")]
extern {
    fn open_serial(name: *const c_char, baud: c_int) -> HANDLE;
    fn read_serial(hfd: HANDLE, buf: *mut u8, buflen: size_t) -> ssize_t;
    fn write_serial(hfd: HANDLE, buf: *const u8, buflen: size_t) -> ssize_t;
    fn close_serial(hfd: HANDLE);
    fn flush_serial(hfd: HANDLE);
}

#[derive(Debug, Clone)]
pub struct SerialDevice {
    hfd: HANDLE,
}

unsafe impl Send for SerialDevice {}

impl  SerialDevice {
    pub fn new() -> Self {
	Self {
	    hfd: INVALID_HANDLE_VALUE,
	}
    }

    pub fn open(&mut self, dname: &str, baud: isize) -> io::Result<()> {
	let dptr = CString::new(dname.to_string()).unwrap();
	unsafe {
	    self.hfd = open_serial(dptr.as_ptr(), baud as c_int);
	    if self.hfd ==  INVALID_HANDLE_VALUE {
		Err(Error::last_os_error())
	    } else {
		Ok(())
	    }
	}
    }

    pub fn clear (&self) {
	unsafe { flush_serial(self.hfd)}
    }

    pub fn close(&mut self)  {
	if self.hfd != INVALID_HANDLE_VALUE {
	    unsafe { close_serial(self.hfd); }
	    self.hfd = INVALID_HANDLE_VALUE;
	}
    }
}

impl io::Read for SerialDevice {
    fn read(&mut self,  buf: &mut [u8]) -> io::Result<usize> {
	let n : ssize_t;
	unsafe { n = read_serial(self.hfd, buf.as_mut_ptr(), buf.len() as size_t); }
	if n == 0 {
	    Err(io::Error::last_os_error())
	} else {
	    Ok(n as usize)
	}
    }
}

impl io::Write for SerialDevice {
    fn write(&mut self, src: &[u8]) ->  io::Result<usize> {
	let n :ssize_t;
	unsafe {
	    n = write_serial(self.hfd, src.as_ptr(), src.len());
	}
	match n {
	    0 => Err(io::Error::last_os_error()),
	    _ => Ok(n as usize),
	}
    }

    fn flush (&mut self) -> io::Result<()> {
	Ok(())
    }
}

impl Drop for SerialDevice {
    fn drop(&mut self) {
        self.close()
    }
}
