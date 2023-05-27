#include <stddef.h>
#include <stdlib.h>
#include <stdint.h>


#if !defined( WIN32 )
extern int open_serial(const char * name, int baudrate);
extern ssize_t read_serial(int fd, uint8_t *buf, size_t buflen);
extern ssize_t write_serial(int fd, uint8_t *buf, size_t buflen);
extern void set_timeout(int fd, int tenths, int number);
extern void close_serial(int fd);
extern void flush_serial(int fd);
#else
#include <windows.h>
extern HANDLE open_serial(const char * name, int baudrate);
extern ssize_t read_serial(HANDLE fd, uint8_t *buf, size_t buflen);
extern ssize_t write_serial(HANDLE fd, uint8_t *buf, size_t buflen);
extern void set_timeout(HANDLE fd, int tenths, int number);
extern void close_serial(HANDLE fd);
extern void flush_serial(HANDLE fd);
#endif
