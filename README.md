# Using rust for MSP (inav etc.)

## Introduction

In the unlikely event that you're curious about using rust to communicate with a MSP flight controller (for example [inav](https://github.com/iNavFlight/inav), betaflight, multiwii even), then here's a trivial example of rust asynchronous MSP (using a "channels" pattern).

Note that this is about day 4 of the author's on/off rust adventure, it may be non-idiomatic, naive etc. PRs welcome.

## Example

Bench test FC, indoors, somewhat random GPS data.

```
$ msptest /dev/rfcomm1
Serial port: /dev/rfcomm1
Name: BenchyMcTest
API Version: 2.4
Firmware: INAV
FW Version: 3.1.0
Git revsion: b079efca
Board: WINGFC
Extant waypoints in FC: 16 of 60, valid true
Voltage: 11.9
GPS: fix 2, sats 5, lat, lon, alt 50.9***** -1.5***** -1, spd 0.33 cog 7.7 hdop 6.59
```

## Usage

```
$ msptest --help
Usage: msptest [options] [device-node]
Version: 0.10.0

Options:
    -s, --slow          slow mode
    -1, --once          Single iteration, then exit
    -v, --version       Show version
    -h, --help          print this help menu
```

On Linux / Macos / Windows / FreeBSD, if no device is given, the application will make a reasonable attempt to evince any valid serial device; e.g. with `msptest` installed on `$PATH`:

```
$ msptest
Serial port: /dev/ttyACM0
Name: BenchyMcTesty
API Version: 2.4
Firmware: INAV
FW Version: 6.0.0
Git revsion: 4bbd2fa5
Board: WINGFC
Extant waypoints in FC: 0 of 120, valid false
Uptime: 550s
Voltage: 0.00
GPS: fix 0, sats 0, lat, lon, alt 0.000000 0.000000 0, spd 0.00 cog 0 hdop 99.99
Elapsed 37.25s 2306 messages, rate 61.90/s
^C
```

^C to exit.

Note for FreeBSD, only /dev/cuaU* is recognised:

Thusly:

```
# This would be auto-discovered
# macos
msptest /dev/cu.usbmodem0x80000001

# Windows
# This would be auto-discovered
msptest.exe COM17
```

## Makefile

As a short cut for `cargo` commands / options, there's a Makefile

* `make build`    :  Builds a release target
* `make install`  :  Builds a release target, installs to ~/.local/bin
* `make debug`    :  Builds a debug target
* `make windows`  :  Cross-compiles a Windows executable on sane host OS
* `make clean`    :  Clean

## Legacy

### MultiWii

```
$ msptest
Serial port: /dev/ttyUSB0
MSP Vers: 241, (protocol v1)
Voltage: 4.20
GPS: fix 0, sats 0, 0.000000° 0.000000° 0m, spd 0.00 cog 0
Elapsed 21.64s 1298 messages, rate 59.99/s
```

### INAV F1 Processor

Note the serial message rate. The changes since 1.7 / 1.8 (increased functionality, changing of task priorities etc.) have not improved serial I/O rates, despite much faster CPUs.

```
$ msptest /dev/ttyACM1
Serial port: /dev/ttyACM1
MSP Vers: 231, (protocol v2)
Name: BV-CC3D
API Version: 2.1
Firmware: INAV
FW Version: 1.9.254
Git revsion: e4510e11
Board: CC3D
Extant waypoints in FC: 0 of 30, valid false
Voltage: 0.00
GPS: fix 0, sats 0, 0.000000° 0.000000° 0m, spd 0.00 cog 0 hdop 0.00
Elapsed 14.31s 1427 messages, rate 99.69/s
```

## TUI Display

The example has now migrated to a TUI application:

```
                                  MSP Test Viewer
                       v0.12.0 on freebsd 13.1-RELEASE x86_64

Port    : /dev/cuaU0
MW Vers : ---
Name    : BenchyMcTesty
API Vers: 2.4 (MSP v2)
FC      : INAV
FC Vers : 6.0.0
Build   : Dec 29 2022 12:38:03 (243b867d)
Board   : WINGFC
WP Info : 0 of 120, valid false
Uptime  : 90563s
Power   : 0.0 volts, 0.11 amps
GPS     : fix 0, sats 0, 0.000000° 0.000000° 0m, 0m/s 0° hdop 99.99
Arming  : NavUnsafe H/WFail RCLink (0x48800)
Rate    : 5587388 messages in 90358.35s (61.8/s) (unknown: 1, crc 0)
```

From 0.12.0, the rate line includes the count of unknown massages and CRC errors. The CRC count should be zero; the unknown count will vary according to version:

```
Board   : MultiWii
Rate    : 1934 messages in 30.9s (62.6/s) (unknown: 649, crc 0)

Board   : CC3D
FC Vers : 1.9.254
Rate    : 4072 messages in 40.8s (99.7/s) (unknown: 1016, crc 0)

Board   : MATEKF405
FC Vers : 6.0.0
Rate    : 2384 messages in 38.5s (61.9/s) (unknown: 1, crc 0)
```

* MultiWii: c. 33% unknown
* Ancient INAV: c. 25% unknown
* Modern INAV: 1 unknown

## Impementation

The rust serialport crate is used device enumeration. Prior to version 0.10.0, the serialport crate was also used for I/O; now the `serial2` is used for I/O, as it "sort of" works on Windows.

serialport performance on Windows is poor (c. 25% of Linux / FreeBSD / Macos) and unreliable across multiple threads. The `serial2` implementation is thread safe and the Windows performance is now around 40% of that of the POSIX platforms. Note that this is a rust limitation; when `msp-s` briefly used a custom, (unsafe {}) 'C' serial reader, the Windows performance was quite close to that of the POSIX platforms.

## Other

There is an [similar Golang example](https://github.com/stronnag/msp-go); you may judge which is the cleanest / simplest, however the rust version is also more capable.

## Licence

MIT, 0BSD or similar.
