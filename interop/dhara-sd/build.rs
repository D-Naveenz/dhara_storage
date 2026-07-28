fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto = "../proto/dhara_sd.proto";
    println!("cargo:rerun-if-changed={proto}");
    // SAFETY: build script is single-threaded before codegen runs.
    unsafe {
        std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path()?);
    }
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile_protos(&[proto], &["../proto"])?;

    // Host OS ≠ target OS when cross-compiling; use CARGO_CFG_TARGET_OS.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set("FileVersion", env!("CARGO_PKG_VERSION"));
        res.set("ProductVersion", env!("CARGO_PKG_VERSION"));
        res.compile()?;
    }

    Ok(())
}
