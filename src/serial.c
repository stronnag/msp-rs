/*
 * Copyright (C) 2014 Jonathan Hudson <jh+mwptools@daria.co.uk>
 *
 * This program is free software; you can redistribute it and/or
 * modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation; either version 3
 * of the License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 59 Temple Place - Suite 330, Boston, MA  02111-1307, USA.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <errno.h>
#include <fcntl.h>
#include <stdbool.h>
#include "serial.h"

#if !defined( WIN32 )
#ifdef  __FreeBSD__
# define __BSD_VISIBLE 1
#endif
#include <sys/ioctl.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <termios.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <netdb.h>
#include <arpa/inet.h>
#include <errno.h>

#ifdef __linux__
#include <linux/serial.h>
#endif

void flush_serial(int fd) {
  tcflush(fd, TCIOFLUSH);
}

static int rate_to_constant(int baudrate) {
#define B(x) case x: return B##x
    switch(baudrate) {
        B(50);     B(75);     B(110);    B(134);    B(150);
        B(200);    B(300);    B(600);    B(1200);   B(1800);
        B(2400);   B(4800);   B(9600);   B(19200);  B(38400);
        B(57600);  B(115200); B(230400);
#ifdef __linux__
        B(460800); B(921600);
        B(500000); B(576000); B(1000000); B(1152000); B(1500000);
#endif
#ifdef __FreeBSD__
        B(460800); B(500000);  B(921600);
        B(1000000); B(1500000);
	B(2000000); B(2500000);
	B(3000000); B(3500000);
	B(4000000);
#endif
	default: return 0;
    }
#undef B
}

int set_fd_speed(int fd, int rate) {
  struct termios tio;
  int res=0;
  int speed = rate_to_constant(rate);

#ifdef __linux__
  if(speed == 0) {
#include <asm/termios.h>
#include <asm/ioctls.h>
    struct termios2 t;
    if((res = ioctl(fd, TCGETS2, &t)) != -1) {
      t.c_ospeed = t.c_ispeed = rate;
      t.c_cflag &= ~CBAUD;
      t.c_cflag |= (BOTHER|CBAUDEX);
      res = ioctl(fd, TCSETS2, &t);
    }
  }
#endif
  if (speed != 0) {
    tcgetattr(fd, &tio);
    if((res = cfsetispeed(&tio,speed)) != -1) {
      res = cfsetospeed(&tio,speed);
      tcsetattr(fd,TCSANOW,&tio);
    }
  }
  return res;
}

int open_serial(const char *device, int baudrate) {
    int fd;
    fd = open(device, O_RDWR|O_NOCTTY);
    if(fd != -1) {
      struct termios tio;
      memset (&tio, 0, sizeof(tio));
      tcgetattr(fd, &tio);
      cfmakeraw(&tio);
      tio.c_cc[VTIME] = 0;
      tio.c_cc[VMIN] = 1;
      tcsetattr(fd,TCSANOW,&tio);
      if(set_fd_speed(fd, baudrate) == -1) {
        close(fd);
        fd = -1;
      }
    }
    return fd;
}

void set_timeout(int fd, int tenths, int number) {
    struct termios tio;
    memset (&tio, 0, sizeof(tio));
    tcgetattr(fd, &tio);
    tio.c_cc[VTIME] = tenths;
    tio.c_cc[VMIN] = number;
    tcsetattr(fd,TCSANOW,&tio);
}

void close_serial(int fd) {
    tcflush(fd, TCIOFLUSH);
    struct termios tio ={0};
    tio.c_iflag &= ~IGNBRK;
    tio.c_iflag |=  BRKINT;
    tio.c_iflag |=  IGNPAR;
    tio.c_iflag &= ~PARMRK;
    tio.c_iflag &= ~ISTRIP;
    tio.c_iflag &= ~(INLCR | IGNCR | ICRNL);
    tio.c_cflag &= ~CSIZE;
    tio.c_cflag |=  CS8;
    tio.c_cflag |=  CREAD;
    tio.c_lflag |=  ISIG;
    tio.c_lflag &= ~ICANON;
    tio.c_lflag &= ~(ECHO | ECHOE | ECHOK | ECHONL);
    tio.c_lflag &= ~IEXTEN;
    tio.c_cc[VTIME] = 0;
    tio.c_cc[VMIN] = 1;
    tcsetattr(fd,TCSANOW,&tio);
    close(fd);
}

ssize_t read_serial(int fd, uint8_t*buffer, size_t buflen) {
  return read(fd, buffer, buflen);
}

ssize_t write_serial(int fd, uint8_t*buffer, size_t buflen) {
  return write(fd, buffer, buflen);
}

#else
#include <windows.h>

static void show_error(DWORD errval) {
  char errstr[1024];
  FormatMessage(FORMAT_MESSAGE_FROM_SYSTEM, NULL, errval,
                MAKELANGID(LANG_NEUTRAL, SUBLANG_DEFAULT), errstr, sizeof(errstr)-1, NULL);
  fprintf(stderr, "Err: %s\n", errstr);
}

void flush_serial(__attribute__ ((unused)) int fd) {
  HANDLE hfd = (HANDLE)_get_osfhandle(fd);
  PurgeComm(hfd, PURGE_RXABORT|PURGE_TXABORT|PURGE_RXCLEAR|PURGE_TXCLEAR);
}

void set_fd_speed(int fd, int baudrate) {
    DCB dcb = {0};
    BOOL res = FALSE;
    HANDLE hfd = (HANDLE)_get_osfhandle(fd);

    dcb.DCBlength = sizeof(DCB);

    if ((res = GetCommState(hfd, &dcb))) {
        dcb.ByteSize=8;
        dcb.StopBits=ONESTOPBIT;
        dcb.Parity=NOPARITY;
        switch (baudrate) {
            case 0:
            case 115200:
                dcb.BaudRate=CBR_115200;
                break;
            case 2400:
                dcb.BaudRate=CBR_2400;
                break;
            case 4800:
                dcb.BaudRate=CBR_4800;
                break;
            case 9600:
                dcb.BaudRate=CBR_9600;
                break;
            case 19200:
                dcb.BaudRate=CBR_19200;
                break;
            case 38400:
                dcb.BaudRate=CBR_38400;
                break;
            case 57600:
                dcb.BaudRate=CBR_57600;
                break;
        }
        res = SetCommState(hfd, &dcb);
    }
}

void set_timeout(int fd, __attribute__ ((unused)) int p0, __attribute__ ((unused)) int p1) {
  HANDLE hfd = (HANDLE)_get_osfhandle(fd);
  COMMTIMEOUTS ctout;
  GetCommTimeouts(hfd, &ctout);
  ctout.ReadIntervalTimeout = MAXDWORD;
  ctout.ReadTotalTimeoutMultiplier = MAXDWORD;
  ctout.ReadTotalTimeoutConstant = MAXDWORD-1;
  SetCommTimeouts(hfd, &ctout);
}

int open_serial(const char *device, int baudrate) {
  int fd = -1;
  HANDLE hfd = CreateFile(device,
                   GENERIC_READ|GENERIC_WRITE,
                   0,
                   NULL,
                   OPEN_EXISTING,
                   FILE_FLAG_OVERLAPPED,
                   NULL);
  if(hfd != INVALID_HANDLE_VALUE) {
    fd = _open_osfhandle((intptr_t)hfd, O_RDWR);
    set_timeout(fd, 0, 0);
    set_fd_speed(fd, baudrate);
  }
  return fd;
}

void close_serial(int fd) {
  close(fd);
}

ssize_t read_serial(int fd, uint8_t*buffer, size_t buflen) {
  HANDLE hfd = (HANDLE)_get_osfhandle(fd);
  DWORD nb= 0;
  OVERLAPPED ovl={0};
  ovl.hEvent =   CreateEvent(NULL, true, false, NULL);
  if (ReadFile (hfd, buffer, buflen, &nb, &ovl) == 0) {
    DWORD eval = GetLastError();
    if (eval == ERROR_IO_PENDING) {
      GetOverlappedResult(hfd, &ovl, &nb, true);
    } else {
      show_error(eval);
    }
  }
  CloseHandle(ovl.hEvent);
  return (ssize_t)nb;
}

ssize_t write_serial(int fd, uint8_t*buffer, size_t buflen) {
  HANDLE hfd = (HANDLE)_get_osfhandle(fd);
  DWORD nb= 0;
  OVERLAPPED ovl={0};
  ovl.hEvent = CreateEvent(NULL, true, false, NULL);
  if (WriteFile (hfd, buffer, buflen, &nb, &ovl) == 0) {
    DWORD eval = GetLastError();
    if (eval == ERROR_IO_PENDING) {
      GetOverlappedResult(hfd, &ovl, &nb, true);
    } else {
      show_error(eval);
    }
  }
  CloseHandle(ovl.hEvent);
  return (ssize_t)nb;
}

#endif
