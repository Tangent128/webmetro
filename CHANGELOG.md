## v0.3.1-dev
- forget a channel's initialization segment when no transmitter is active. This improves behavior when a channel is occasionally used for streams with different codecs.
- Add INFO logging for channel creation/garbage-collection
- Start throttle timing on first data instead of throttle creation (improves cases where the source is slow to start)

## v0.3.0
- update internals to v0.2 of `warp` and `tokio`; no remaining code relies on `futures` 0.1

## v0.2.2
- use the `log` and `env_logger` crates for logging; the `RUST_LOG` environment variable configures the logging level.
  - see [the env_logger documentation](https://docs.rs/env_logger/*/env_logger/) for more information
- support listening on multiple addresses if given a DNS name instead of an IP address. All bindings reference the same namespace for channels, but this allows, e.g., binding to both IPv4 and IPv6 `localhost`.
- released November 20, 2019

## v0.2.1
- update most internals to use `std::future`

## v0.2.0
- support proxying an arbitrary number of streams at `/live/$NAME`
- released October 27, 2018
