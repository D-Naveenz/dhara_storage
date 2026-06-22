# dharastorage

`dharastorage` exposes a path-based native C ABI over `dhara_storage`.

It is the supported interop layer for `src/bindings/Dhara.Storage` and for any
other host that wants to consume the Rust runtime through a stable UTF-8 oriented
ABI instead of linking Rust types directly.

## Surface

- immediate query functions for analysis, metadata, listings, reads, writes, and path mutations
- background operation handles for copy, move, delete, read, and write workflows with progress and cancellation
- directory watch handles with debounced typed native events
- streaming write sessions for chunked uploads from managed hosts
- explicit buffer-free helpers for owned strings and byte arrays
- native logger registration for forwarding structured `tracing` events into a host environment

## ABI Policy

The official structured-result ABI is typed and C-compatible:

- `dhara_analyze_path`
- `dhara_get_file_info`
- `dhara_get_directory_info`
- `dhara_list_files`
- `dhara_list_directories`
- `dhara_list_entries`
- `dhara_watch_try_recv_event`
- `dhara_watch_recv_event`
- `dhara_watch_recv_event_timeout`

These functions return Rust-owned `#[repr(C)]` result handles containing only
fixed-layout fields, UTF-8 pointer/length slices, typed pointer/length arrays,
and fixed integer discriminants. Callers must copy what they need immediately
and release the result with the matching `*_free` function.

Legacy JSON structured-result exports are kept only as temporary compatibility
shims and are named with `_json_old`, for example `dhara_analyze_path_json_old`.
They are deprecated and can be removed mechanically once no host imports an
`_old` symbol. JSON remains acceptable for cold-path errors and logging records.

## Design Notes

- String inputs are UTF-8 and null-terminated.
- Hot structured results cross the ABI through typed Rust-owned handles.
- Strings inside typed results cross as UTF-8 pointer/length slices, not embedded C# strings.
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
