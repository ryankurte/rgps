# rgps(c|d)

A GPS daemon with dynamic NTRIP support (written in Rust).
This is intended to provide a simpler alternative to [gpsd](https://gpsd.gitlab.io/gpsd/) and enable setups that -just work- with DGPS or RTK-GPS on embedded linux devices (particularly robots).

This provides a `rgpsd` service which can be configured to talk to GPS', NTRIP servers, and other location-related things, and a `rgpsc` client that can be used to request or stream status messages from the daemon (and a rust client library that can be used from your own code).

## Status

This project is a work in progress. Unless you have a (particular|peculiar) reason to do so you should probably use the much better established [gpsd](https://gpsd.gitlab.io/gpsd/).

[![GitHub tag](https://img.shields.io/github/tag/ryankurte/rgps.svg)](https://github.com/ryankurte/rgps)
![Build Status](https://github.com/ryankurte/rgps/workflows/CI/badge.svg)

### Components

- [rgps-client](./client) provides a client library and command line interface for interacting with the daemon
- [rgps-daemon](./daemon) provides the RGPS daemon implementation
- [rgps-core](./core) defines common GPS messages used to communicate with the daemon

## Usage

TODO.

## Features

- [x] Cross-platform support, fully packaged for Ubuntu/Debian linux, with binaries available for (almost) everything else.
- [x] D-GPS with dynamic NTRIP discovery and updates. If you have an NTRIP/SNIP provider set the daemon will automatically discover (and update) itself to use the best mount as you move through space.
- [x] Multi-GPS support (connect as many or as few GPS' as you fancy).
- [ ] RTK-GPS
  - [ ] Survey In
  - [ ] RTCM sending
  - [ ] RTCM receiving
- [ ] GPS time synchronisation (probably Linux only)
- [ ] Multiple connectors for accessing or subscribing to GPS information
  - [x] Unix socket (Messages)
  - [ ] TCP (Messages or raw GPS|NMEA)
  - [ ] HTTP (Current State)
  - [ ] (Partial) GPSD compatibility, allowing tools that expect to talk to GPSD sockets to use RGPSD instead
- [ ] mDNS service registration (GPS position, RTK/RTCM)

See the [issues](https://github.com/ryankurte/rgps/issues) for more details.

#### Anti-features

This will not:
- Dynamically discover GPS devices or perform auto-baud detection. I'm pretty comfortable with knowing the number of GPS' on a platform and having them configured ahead-of-time.
- Support device hot-plug* (*though we should have connection recovery in case of (dis|re)connection)
- Allow clients to fundamentally change the operation of connected GPS'
  - While triggering an RTK survey or swapping between DGPS and RTK modes should be supported over the control socket, most subscribers to GPS 


