use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=package");

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let source_dir = manifest_dir.join("package");
    if !source_dir.exists() {
        return;
    }

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let profile = env::var_os("PROFILE").expect("PROFILE");
    let Some(artifact_dir) = profile_artifact_dir(&out_dir, &profile) else {
        panic!(
            "failed to resolve artifact directory from OUT_DIR '{}'",
            out_dir.display()
        );
    };

    let destination_dir = artifact_dir.join("package");
    recreate_directory(&destination_dir).expect("recreate package output directory");
    copy_directory_recursive(&source_dir, &destination_dir).expect("copy package directory");
}

fn profile_artifact_dir(out_dir: &Path, profile: &OsStr) -> Option<PathBuf> {
    out_dir
        .ancestors()
        .find(|path| path.file_name() == Some(profile))
        .map(Path::to_path_buf)
}

fn recreate_directory(path: &Path) -> io::Result<()> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    fs::create_dir_all(path)
}

fn copy_directory_recursive(source: &Path, destination: &Path) -> io::Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let entry_type = entry.file_type()?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());

        if entry_type.is_dir() {
            fs::create_dir_all(&destination_path)?;
            copy_directory_recursive(&source_path, &destination_path)?;
        } else if entry_type.is_file() {
            fs::copy(&source_path, &destination_path)?;
        }
    }

    Ok(())
}
