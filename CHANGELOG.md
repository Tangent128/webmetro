## v0.2.2
- support listening on multiple addresses if given a DNS name instead of an IP address. All bindings reference the same namespace for channels, but this allows, e.g., binding to both IPv4 and IPv6 `localhost`.
- released November 20, 2019

## v0.2.1
- update most internals to use `std::future`

## v0.2.0
- support proxying an arbitrary number of streams at `/live/$NAME`
- released October 27, 2018
