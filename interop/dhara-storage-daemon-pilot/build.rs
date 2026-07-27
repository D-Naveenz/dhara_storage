fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto = "../proto/dhara_pilot.proto";
    println!("cargo:rerun-if-changed={proto}");
    // SAFETY: build script is single-threaded before codegen runs.
    unsafe {
        std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path()?);
    }
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile_protos(&[proto], &["../proto"])?;
    Ok(())
}
