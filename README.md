<div align="center">

<img src="src/bindings/csharp/Dhara.Storage/assets/dhara-logo-colored_sm.png" alt="Dhara Storage" width="120" />

# Dhara Storage

[![dhara_storage](https://img.shields.io/crates/v/dhara_storage?label=dhara_storage)](https://crates.io/crates/dhara_storage)
[![dhara_storage_core](https://img.shields.io/crates/v/dhara_storage_core?label=dhara_storage_core)](https://crates.io/crates/dhara_storage_core)
[![Dhara.Storage](https://img.shields.io/nuget/v/Dhara.Storage?label=Dhara.Storage)](https://www.nuget.org/packages/Dhara.Storage)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE.txt)

**Cross-platform local storage for applications** — file and directory handles, watching, metadata, and **content-based file intelligence** (not a guess from the extension).

</div>

Ordinary file APIs stop at paths and bytes. Many “content type” fields stop at the file name. Dhara combines a storage-handle model with signature-based analysis in one runtime, so apps and libraries can classify and manage files without a separate MIME stack—and without being locked to one OS app model.

The core is written in Rust for memory safety, thread safety, and fearless concurrency under heavy I/O and analysis workloads.

## Why use it

- Know the real type from file bytes (bundled definitions), not just the extension
- File and directory handles with transfers that support progress and cancellation
- Built-in directory watching and shell-aware metadata where the OS provides it
- Works from libraries and desktop apps on Windows, Linux, and macOS
- Native core when concurrency and system-level correctness matter

## Choose your package

Pick the surface that matches how you build. Each link is the install-and-usage guide for that package.

| If you are… | Use | Guide |
|-------------|-----|-------|
| Building in **Rust** | `dhara_storage` | [crates.io / README][readme-dhara-storage] |
| Working with **file definition** data | `dhara_storage_core` | [crates.io / README][readme-core] |
| Integrating via **C ABI** | `dharastorage` | [FFI README][readme-dharastorage] |
| Building in **.NET** | `Dhara.Storage` | [NuGet / README][readme-nuget] |
| Operating releases / defs / packaging | `drot` | [DROT README][readme-tool] |

## Showcase

<div align="center">

Published packages track the current workspace line on [crates.io][crates-dhara] and [NuGet][nuget-dhara].

*Screenshots and benchmarks — coming soon.*

</div>

## Contributing

Open a pull request against `main`. Keep **project** READMEs accurate when public behavior or publish surfaces change.

Deep technical reference: [docs index][docs-index].  
Contributor / agent context: [AGENTS.md][agents].

## License

Licensed under [Apache-2.0][license].

[readme-dhara-storage]: https://github.com/D-Naveenz/dhara_storage/blob/main/src/core/dhara_storage/README.md
[readme-core]: https://github.com/D-Naveenz/dhara_storage/blob/main/src/core/dhara_storage_core/README.md
[readme-dharastorage]: https://github.com/D-Naveenz/dhara_storage/blob/main/src/bindings/dharastorage-ffi/README.md
[readme-nuget]: https://github.com/D-Naveenz/dhara_storage/blob/main/src/bindings/csharp/Dhara.Storage/README.md
[readme-tool]: https://github.com/D-Naveenz/dhara_storage/blob/main/tooling/drot/README.md
[crates-dhara]: https://crates.io/crates/dhara_storage
[nuget-dhara]: https://www.nuget.org/packages/Dhara.Storage
[docs-index]: docs/README.md
[agents]: AGENTS.md
[license]: LICENSE.txt
