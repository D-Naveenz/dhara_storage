# Windows code signing for dhara-sd (OSS)

`dhara-sd.exe` embeds VERSIONINFO via **`winresource`** (not the unmaintained `winres` crate). That fills Product name, version, copyright, and `OriginalFilename` in Explorer.

## What VERSIONINFO does not do

[Smart App Control](https://learn.microsoft.com/en-us/windows/apps/develop/smart-app-control/code-signing-for-smart-app-control) and many Defender “unknown publisher” prompts require **Authenticode** with an **RSA** certificate from a trusted CA (or Microsoft Trusted Signing). File properties metadata is not a publisher identity.

## What open-source projects usually do

1. Ship unsigned + document SmartScreen / SAC warnings (most small tools).
2. Add VERSIONINFO for professionalism.
3. When distributing broadly: OV/EV code-signing cert, or **Microsoft Trusted Signing** in CI.
4. Prefer trusted channels (winget, Store) when available.

Do **not** document “turn off Smart App Control” as a product requirement. Developers may disable SAC locally so unsigned toolchains (`cargo`, `rustc`) run; that is a workstation choice, not an end-user instruction.

## Follow-up (Next milestone)

Wire `signtool` / Trusted Signing into release CI for `dhara-sd` (and eventually `drot`) before marketing the NuGet sidecar to SAC-On machines.
