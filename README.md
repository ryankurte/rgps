# rgps(c|d)

A GPS daemon with dynamic NTRIP support (written in Rust).
This is intended to provide a simpler alternative to [gpsd](https://gpsd.gitlab.io/gpsd/) and enable setups that -just work- with DGPS or RTK-GPS on embedded linux devices (particularly robots).


## Status

This project is a work in progress. Unless you have a (particular|peculiar) reason to do so you should probably use the much better established [gpsd](https://gpsd.gitlab.io/gpsd/).

[![GitHub tag](https://img.shields.io/github/tag/ryankurte/rgps.svg)](https://github.com/ryankurte/rgps)
![Build Status](https://github.com/ryankurte/rgps/workflows/CI/badge.svg)


## Features

- [x] Cross-platform support, fully packaged for Ubuntu/Debian linux, with binaries available for (almost) everything else.
- [x] D-GPS with dynamic NTRIP discovery and updates. If you have an NTRIP/SNIP provider set the daemon will automatically discover (and update) itself to use the best mount.
- [ ] RTK-GPS
  - [ ] Survey In (Ublox, Quectel)
  - [ ] RTCM sending
  - [ ] RTCM receiving
- [ ] GPS time synchronisation
- [ ] Multiple connectors for accessing or subscribing to GPS information
  - [ ] Unix socket
  - [ ] TCP (Messages or raw GPS)
  - [ ] HTTP (Messages)
  - [ ] (Partial) GPSD compatibility, allowing tools that expect to talk to GPSD sockets to use RGPSD instead

See the [issues](https://github.com/ryankurte/rgps/issues) for more details.

#### Anti-features

This will not:
- Dynamically discover GPS devices or perform auto-baud detection. I'm pretty comfortable with knowing the number of GPS' on a platform and having them configured ahead-of-time.
- Support device hot-plug* (*though we should have connection recovery in case of intermittent disconnection)


## Components

- [rgpsc](./ctl) provides a client library and command line interface for interacting with the daemon
- [rgpsd](./daemon) provides the RGPS daemon implementation
- [rgps](./proto) defines common GPS messages used to communicate with the daemon

