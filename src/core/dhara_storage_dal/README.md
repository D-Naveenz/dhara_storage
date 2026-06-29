# dhara_storage_dal

`dhara_storage_dal` is the FlatBuffers-backed data access layer for Dhara
Storage file definitions.

It owns the internal `filedefs.dat` artifact format, the file-definition model
types, and the serializer/deserializer shared by `dhara_storage` and
`dhara_tool`.

For the full on-disk format specification, see
[`docs/filedefs-dat.md`](../../../docs/filedefs-dat.md).

The runtime `filedefs.dat` package lives at `resources/filedefs.dat` in this crate and is
embedded at compile time via `include_bytes!`. Refresh it with:

```powershell
cargo run -p dhara_tool -- defs sync-embedded
```

Regenerate Rust FlatBuffers accessors after editing the schema:

```powershell
flatc --rust -o src/core/dhara_storage_dal/src/generated src/core/dhara_storage_dal/schema/filedefs.fbs
```
