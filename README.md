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
$ target/release/msptest --help
Usage: msptest [options] DEVICE

Options:
    -m, --mspvers 2     set msp version
    -1, --once          exit after one iteration
    -s, --slow          slow mode
    -h, --help          print this help menu
```

On POSIX platforms at least, if no device is given, the application will make a reasonable attempt to evince any valid serial device, with `msptest` installed on `$PATH`:

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

However, for non-Linux, it may be necessary to define the device node, e.g. FreeBSD:

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
# macos
msptest /dev/cu.usbmodem0x80000001

# Windows
msptest.exe COM17
```

Note that on Windows, the rust `serialport` performance is less than impressive.

## Makefile

As a short cut for `cargo` commands / options, there's a Makefile

* `make build`    :  Builds a release target
* `make install`  :  Builds a release target, installs to ~/.local/bin
* `make debug`    :  Builds a debug target
* `make windows`  :  Cross-compiles a Windows executable on sane host OS
* `make clean`    :  Clean

## Legacy

### MultiWii

Requires `-m 1` to force MSP v1.

```
$ msptest -m 1
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

## Licence

MIT, 0BSD or similar.
