# Using rust for MSP (inav etc.)

## Introduction

In the unlikely event that you're curious about using rust to commuicate with a MSP flight controller (for example [inav](https://github.com/iNavFlight/inav), betaflight, multiwii even), then here's a trivial example of rust asynchronous MSP (using a "channels" pattern).

Note that this is about day 2 of the author's rust adventure, it may be non-idiomatic, naive etc. PRs welcome.

## Example

Bench test FC, indoors, somewhat random GPS data.

```
$ target/release/msptest /dev/rfcomm1
Serial port: /dev/rfcomm1
MSP Vers: 231, (protocol v2)
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
Usage: target/release/msptest [options] DEVICE

Options:
    -m, --mspvers 2     set msp version
    -h, --help          print this help menu

```

On POSIX platforms at least, if no device is given, the application will make a reasonable attempt to evince any valid serial device.

```
$ target/release/msptest
Serial port: /dev/ttyACM0
...

```

## Licence

MIT or similar.
