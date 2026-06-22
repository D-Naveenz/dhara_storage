# dhara_storage_dal

`dhara_storage_dal` is the FlatBuffers-backed data access layer for Dhara
Storage file definitions.

It owns the internal `filedefs.dat` artifact format, the file-definition model
types, and the serializer/deserializer shared by `dhara_storage` and
`dhara_storage_ops`.

Regenerate Rust FlatBuffers accessors after editing the schema:

```powershell
flatc --rust -o src/static/dhara_storage_dal/src/generated src/static/dhara_storage_dal/schema/filedefs.fbs
```
