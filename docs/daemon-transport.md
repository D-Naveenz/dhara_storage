# Daemon transport (dhara-sd)

Foreign-language bindings use **`dhara-sd`** as a sidecar: gRPC for control, OS handle/FD transfer for read/write bytes.

## Windows

| Concern | Mechanism |
|---------|-----------|
| Control plane | Named pipe + HTTP/2 gRPC (`tonic` / `Grpc.Net.Client`) |
| Default pipe | `\\.\pipe\dhara-sd` (override via CLI arg / host options) |
| Data plane | `OpenReadHandle` / `OpenWriteHandle` → `DuplicateHandle` into the host after `Handshake(parent_pid)` |
| Accept loop | Create the **next** listening instance **before** yielding a connected pipe |
| Host stdio | Do **not** redirect unread daemon stdout/stderr (full pipe blocks the process) |

VERSIONINFO is embedded with [`winresource`](https://crates.io/crates/winresource). That does **not** satisfy Smart App Control; release signing (Authenticode / Trusted Signing) is a later milestone. Do not ask users to disable SAC.

## Linux / macOS

| Concern | Mechanism |
|---------|-----------|
| Control plane | Unix domain socket (`grpc.sock`) + HTTP/2 gRPC |
| Data plane | `SCM_RIGHTS` FD passing on a dedicated `fd.sock` after `Handshake` |
| Endpoint layout | Host passes a directory; daemon creates `{dir}/grpc.sock` and `{dir}/fd.sock` |
| CI | Stage `dhara-sd` per RID; managed tests run on Linux under `xvfb-run` |

## Logging

Daemon logging is owned by **`dhara-sd`**: one `tracing_subscriber` captures **`dhara_storage`** and **`dhara-sd`** events and fans them out on the parallel **`StreamLogs`** gRPC stream. The C# host maps each record's Rust `target` string to an `ILogger` category. Do not scrape stdout/stderr.

## Shell metadata

`GetFileInfo` / `GetDirectoryInfo` accept `include_shell_details`, `include_icon`, and `icon_size`. Icons are RGBA bytes in the unary RPC response (not shared memory).

## Product vs contrast RPCs

Prefer handle/FD open for host-native I/O. `ReadFileBytes` / `WriteFileBytes` remain for benchmarks that measure bytes-over-gRPC tax — not the recommended app path.

## Related

- Protocol: [`interop/proto/dhara_sd.proto`](../interop/proto/dhara_sd.proto)
- Binary: [`interop/dhara-sd/`](../interop/dhara-sd/)
- Binding benchmarks: [binding-benchmarks.md](binding-benchmarks.md)
- Code signing note: [windows-code-signing.md](windows-code-signing.md)
