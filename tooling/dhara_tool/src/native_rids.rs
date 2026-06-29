use anyhow::{Result, bail};

/// Native library file name placed under `runtimes/{rid}/native/`.
pub fn native_lib_filename(rid: &str) -> Result<&'static str> {
    match rid {
        "win-x64" | "win-arm64" => Ok("dharastorage.dll"),
        "linux-x64" | "linux-arm64" => Ok("libdharastorage.so"),
        "osx-arm64" => Ok("libdharastorage.dylib"),
        _ => bail!("unsupported runtime identifier for native library name: {rid}"),
    }
}

/// Package-relative path for a native library entry inside a `.nupkg`.
pub fn package_native_path(rid: &str) -> Result<String> {
    Ok(format!(
        "runtimes/{rid}/native/{}",
        native_lib_filename(rid)?
    ))
}

/// RIDs that can be built on the current host OS without cross-compilation.
pub fn buildable_runtimes_on_host(all_runtimes: &[String]) -> Vec<String> {
    let host_os = std::env::consts::OS;
    all_runtimes
        .iter()
        .filter(|rid| is_rid_buildable_on_host(rid, host_os))
        .cloned()
        .collect()
}

fn is_rid_buildable_on_host(rid: &str, host_os: &str) -> bool {
    match host_os {
        "windows" => matches!(rid, "win-x64" | "win-arm64"),
        "linux" => matches!(rid, "linux-x64" | "linux-arm64"),
        "macos" => rid == "osx-arm64",
        _ => false,
    }
}

/// MSBuild `Platform` value when required for a RID.
pub fn platform(runtime: &str) -> Result<Option<&'static str>> {
    match runtime {
        "win-x64" => Ok(Some("x64")),
        "win-arm64" => Ok(Some("ARM64")),
        "win-x86" => Ok(Some("x86")),
        "linux-x64" | "linux-arm64" | "osx-arm64" => Ok(None),
        _ => bail!("unsupported runtime for Platform inference: {runtime}"),
    }
}

/// MSBuild `PlatformTarget` value when required for a RID.
pub fn platform_target(runtime: &str) -> Result<Option<&'static str>> {
    match runtime {
        "win-x64" => Ok(Some("x64")),
        "win-arm64" => Ok(Some("arm64")),
        "win-x86" => Ok(Some("x86")),
        "linux-x64" | "linux-arm64" | "osx-arm64" => Ok(None),
        _ => bail!("unsupported runtime for PlatformTarget inference: {runtime}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_native_paths_use_platform_extensions() {
        assert_eq!(
            package_native_path("win-x64").unwrap(),
            "runtimes/win-x64/native/dharastorage.dll"
        );
        assert_eq!(
            package_native_path("linux-arm64").unwrap(),
            "runtimes/linux-arm64/native/libdharastorage.so"
        );
        assert_eq!(
            package_native_path("osx-arm64").unwrap(),
            "runtimes/osx-arm64/native/libdharastorage.dylib"
        );
    }
}
