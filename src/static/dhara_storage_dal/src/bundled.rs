use once_cell::sync::Lazy;
use tracing::debug;

use crate::codec::decode_definition_package;
use crate::error::DefinitionPackageError;
use crate::model::DefinitionPackage;

const BUNDLED_FILEDEFS_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../tooling/output/filedefs.dat"
));

static PACKAGE: Lazy<Result<DefinitionPackage, String>> =
    Lazy::new(|| decode_definition_package(BUNDLED_FILEDEFS_BYTES).map_err(|err| err.to_string()));

/// Returns the embedded file-definition package bundled with the DAL crate.
///
/// # Errors
///
/// Returns an error when the embedded `filedefs.dat` asset cannot be decoded.
pub fn bundled_definition_package() -> Result<&'static DefinitionPackage, DefinitionPackageError> {
    debug!(target: "dhara_storage_dal", "loading bundled definition package");
    PACKAGE
        .as_ref()
        .map_err(|message| DefinitionPackageError::BundledLoad {
            message: message.clone(),
        })
}

#[cfg(test)]
mod tests {
    use super::bundled_definition_package;

    #[test]
    fn bundled_dat_loads_successfully() {
        let package =
            bundled_definition_package().expect("bundled definitions package should decode");

        assert!(package.package_version.starts_with("trid-"));
        assert!(!package.definitions.is_empty());
    }
}
