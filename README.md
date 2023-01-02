# Using rust for MSP (inav etc.)

## Introduction

In the unlikely event that you're curious about using rust to commuicate with a MSP flight controller (for example [inav](https://github.com/iNavFlight/inav), betaflight, multiwii even), then here's a trivial example of rust asynchronous MSP (using a "channels" pattern).

Note that this is about day 3 of the author's on/off rust adventure, it may be non-idiomatic, naive etc. PRs welcome.

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

On Linux / Macos / Windows, if no device is given, the application will make a reasonable attempt to evince any valid serial device; e.g. with `msptest` installed on `$PATH`:

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

However, for FreeBSD, it is currently be necessary to define the device node, e.g. FreeBSD:

```
$ msptest /dev/cuaU0
Serial port: /dev/cuaU0
Name: BenchyMcTesty
API Version: 2.4
Firmware: INAV
FW Version: 6.0.0
Git revsion: 4bbd2fa5
Board: WINGFC
Extant waypoints in FC: 0 of 120, valid false
Uptime: 69s
Voltage: 0.00
GPS: fix 0, sats 0, lat, lon, alt 0.000000 0.000000 0, spd 0.00 cog 0 hdop 99.99
Elapsed 48.81s 3020 messages, rate 61.88/s
```

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

Port    : /dev/ttyACM0
MW Vers : ---
Name    : BenchyMcTesty
API Vers: API Version: 2.4
FC      : INAV
FC Vers : 6.0.0
Build   : d59b1036
Board   : WINGFC
WP Info : Extant waypoints in FC: 0 of 120, valid false
Uptime  : Uptime: 204s
Power   : 0.00 volts
GPS     : fix 0, sats 0, 0.000000° 0.000000° 0m, 0m/s 0° hdop 99.99
Arming  : NavUnsafe H/WFail
Rate    : Elapsed 11.90s 738 messages, rate 62.00/s
```
## Discussion

### Unsafe (C) serial implementation

This example uses an (unsafe) C language implementation for serial I/O, rather than the serialport crate:

* serialport does not support RISC-V
* serialport performance on Windows is poor (c. 25% of Linux / FreeBSD / Macos). With the C implementation, the Windows performance is about 80% of the POSIX platforms.

Prior to version 0.10.0, the serialport crate was also used for I/O.

### Other

There is an [similar Golang example](https://github.com/stronnag/msp-go); you may judge which is the cleanest / simplest, however the rust version is also slightly more capable.

## Licence

MIT, 0BSD or similar.
