# Using rust for MSP (inav etc.)

## Introduction

In the unlikely event that you're curious about using rust to commuicate with a MSP flight controller (for example [inav](https://github.com/iNavFlight/inav), betaflight, multiwii even), then here's a trivial example of rust asynchronous MSP (using a "channels" pattern).

Note that this is about day 2 of the author's rust adventure, it may be non-idiomatic, naive etc. PRs welcome.

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

## Terminfo

`msptest` uses `terminfo` for cursor addressing. On some platforms, it may be necessary to define where to find the `terminfo` data. e.g. FreeBSD, also defining the device node:

```
$ TERMINFO=/usr/local/share/terminfo msptest /dev/cuaU0
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

### Non-POSIX OS

On non-POSIX OS (i.e. Windows), it will be necessary to define both `TERM` and `TERMINFO` and provide a TERMINFO data file. The canonical `ms-terminal` should work in Windows Terminal, for example (powershell):

Once:
```
mkdir terminfo
mkdir terminfo/m
cp <somepath>\ms-terminal terminfo/m
```
Then, something like:

```
$env:TERM = 'ms-terminal`
$env:TERMINFO = <pathto>\terminfo
msptest COM3
```


## Licence

MIT or similar.
