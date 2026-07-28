# Daemon transport (dhara-sd)

Foreign-language bindings use **`dhara-sd`** as a sidecar: gRPC for control, OS handle/FD transfer for read/write bytes.

## Windows (shipped)

| Concern | Mechanism |
|---------|-----------|
| Control plane | Named pipe + HTTP/2 gRPC (`tonic` / `Grpc.Net.Client`) |
| Default pipe | `\\.\pipe\dhara-sd` (override via CLI arg / host options) |
| Data plane | `OpenReadHandle` / `OpenWriteHandle` → `DuplicateHandle` into the host after `Handshake(parent_pid)` |
| Accept loop | Create the **next** listening instance **before** yielding a connected pipe |
| Host stdio | Do **not** redirect unread daemon stdout/stderr (full pipe blocks the process) |

VERSIONINFO is embedded with [`winresource`](https://crates.io/crates/winresource). That does **not** satisfy Smart App Control; release signing (Authenticode / Trusted Signing) is a later milestone. Do not ask users to disable SAC.

## Linux / macOS (planned)

| Concern | Mechanism |
|---------|-----------|
| Control plane | Unix domain socket + HTTP/2 gRPC |
| Data plane | Pass open FDs with `SCM_RIGHTS` |
| CI | Stage `dhara-sd` per RID alongside (then instead of) the legacy cdylib |

Non-Windows `dhara-sd` currently exits with a clear message until UDS lands.

## Product vs contrast RPCs

Prefer handle/FD open for host-native I/O. `ReadFileBytes` / `WriteFileBytes` remain for benchmarks that measure bytes-over-gRPC tax — not the recommended app path.

## Related

- Protocol: [`interop/proto/dhara_sd.proto`](../interop/proto/dhara_sd.proto)
- Binary: [`interop/dhara-sd/`](../interop/dhara-sd/)
- Binding benchmarks: [binding-benchmarks.md](binding-benchmarks.md)
- Code signing note: [windows-code-signing.md](windows-code-signing.md)
