# dharastorage

`dharastorage` exposes a path-based native C ABI over `dhara_storage`.

It is the supported interop layer for `src/bindings/Dhara.Storage` and for any
other host that wants to consume the Rust runtime through a stable UTF-8 oriented
ABI instead of linking Rust types directly.

## Surface

- immediate query functions for analysis, metadata, listings, reads, writes, and path mutations
- background operation handles for copy, move, delete, read, and write workflows with progress and cancellation
- directory watch handles with debounced JSON events
- streaming write sessions for chunked uploads from managed hosts
- explicit buffer-free helpers for owned strings and byte arrays
- native logger registration for forwarding structured `tracing` events into a host environment

## Design Notes

- String inputs are UTF-8 and null-terminated.
- Rich results are serialized as JSON for simplicity and host ergonomics.
- Raw file reads cross the ABI as owned byte buffers.
- The crate stays intentionally thin; filesystem behavior remains owned by `dhara_storage`.

## Logging

Hosts can register a logger callback through `dhara_register_logger`. Each callback
invocation receives a UTF-8 JSON record with:

- log level
- target/category
- rendered message
- timestamp
- file/module/line information when available
- structured event fields captured from Rust `tracing`

## Related Docs

- Core runtime: <https://github.com/D-Naveenz/dhara_storage/tree/main/src/static/dhara_storage>
- .NET wrapper: <https://github.com/D-Naveenz/dhara_storage/tree/main/src/bindings/Dhara.Storage>
