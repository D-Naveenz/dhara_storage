# dhara_storage_dal

`dhara_storage_dal` is the FlatBuffers-backed data access layer for Dhara
Storage file definitions.

It owns the internal `filedefs.dat` artifact format, the file-definition model
types, and the serializer/deserializer shared by `dhara_storage` and
`dhara_tool`.

The runtime `filedefs.dat` package lives at `tooling/output/filedefs.dat` and is
embedded into this crate at compile time. Refresh it with:

```powershell
cargo run -p dhara_tool -- defs sync-embedded
```

Regenerate Rust FlatBuffers accessors after editing the schema:

```powershell
flatc --rust -o src/static/dhara_storage_dal/src/generated src/static/dhara_storage_dal/schema/filedefs.fbs
```
