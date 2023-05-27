use libc::{c_char, c_int, size_t, ssize_t};
use std::ffi::CString;
use std::io;
use std::io::Error;

#[link(name = "serial")]
extern "C" {
    fn open_serial(name: *const c_char, baud: c_int) -> c_int;
    fn read_serial(fd: c_int, buf: *mut u8, buflen: size_t) -> ssize_t;
    fn write_serial(fd: c_int, buf: *const u8, buflen: size_t) -> ssize_t;
    fn close_serial(fd: c_int);
    fn flush_serial(fd: c_int);
}

#[derive(Debug, Clone)]
pub struct SerialDevice {
    pub fd: c_int,
}

unsafe impl Sync for SerialDevice {}

impl SerialDevice {
    pub fn new() -> Self {
        Self { fd: -1 }
    }

    pub fn open(&mut self, dname: &str, baud: isize) -> io::Result<()> {
        let dptr = CString::new(dname.to_string()).unwrap();
        unsafe {
            self.fd = open_serial(dptr.as_ptr(), baud as c_int);
            if self.fd < 0 {
                Err(Error::last_os_error())
            } else {
                Ok(())
            }
        }
    }

    pub fn close(&mut self) {
        if self.fd != -1 {
            unsafe {
                close_serial(self.fd);
            }
            self.fd = -1;
        }
    }

    pub fn clear(&self) {
        unsafe {
            flush_serial(self.fd);
        }
    }
}

impl io::Read for SerialDevice {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n: ssize_t;
        unsafe {
            n = read_serial(self.fd, buf.as_mut_ptr(), buf.len() as size_t);
        }
        if n <= 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(n as usize)
        }
    }
}

impl io::Write for SerialDevice {
    fn write(&mut self, src: &[u8]) -> io::Result<usize> {
        let n: ssize_t;
        unsafe {
            n = write_serial(self.fd, src.as_ptr(), src.len());
        }
	if n <= 0 {
            Err(io::Error::last_os_error())
	} else {
            Ok(n as usize)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for SerialDevice {
    fn drop(&mut self) {
        self.close();
    }
}
