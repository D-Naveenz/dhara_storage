use std::collections::BTreeSet;

use dhara_storage_dal as dal;
use once_cell::sync::Lazy;
use tracing::{debug, info};

use crate::error::StorageError;

/// Four-byte FlatBuffers file identifier used by normalized file-definition packages.
pub const DEFINITION_PACKAGE_ID: [u8; 4] = *b"FDEF";
const CATCH_ALL_INDEX: usize = 256;

pub use dal::{
    DefinitionPackage, DefinitionPackageError as DefinitionPackageDecodeError, DefinitionRecord,
    SignatureDefinition, SignaturePattern,
};

static DATABASE: Lazy<Result<DefinitionDatabase, String>> = Lazy::new(|| {
    bundled_definition_package()
        .map(DefinitionDatabase::from_package)
        .map_err(|err| err.to_string())
});

/// Indexed in-memory database used by the analysis engine to narrow definition candidates quickly.
#[derive(Debug, Clone)]
pub struct DefinitionDatabase {
    definitions: Vec<DefinitionRecord>,
    pattern_index: Vec<Vec<usize>>,
}

impl DefinitionDatabase {
    fn from_package(package: &DefinitionPackage) -> Self {
        let definitions = package.definitions.clone();
        let mut pattern_index = vec![Vec::new(); CATCH_ALL_INDEX + 1];
        for (idx, definition) in definitions.iter().enumerate() {
            if definition.signature.patterns.is_empty() {
                pattern_index[CATCH_ALL_INDEX].push(idx);
                continue;
            }

            for pattern in &definition.signature.patterns {
                if let Some(first_byte) = pattern.data.first() {
                    pattern_index[*first_byte as usize].push(idx);
                } else {
                    pattern_index[CATCH_ALL_INDEX].push(idx);
                }
            }
        }

        Self {
            definitions,
            pattern_index,
        }
    }

    pub(crate) fn candidate_indices(&self, header: &[u8]) -> Vec<usize> {
        let mut candidates = BTreeSet::new();

        for idx in &self.pattern_index[CATCH_ALL_INDEX] {
            candidates.insert(*idx);
        }

        for (position, byte) in header.iter().enumerate() {
            for definition_idx in &self.pattern_index[*byte as usize] {
                let definition = &self.definitions[*definition_idx];
                if definition
                    .signature
                    .patterns
                    .iter()
                    .any(|pattern| pattern.position as usize == position)
                {
                    candidates.insert(*definition_idx);
                }
            }
        }

        candidates.into_iter().collect()
    }

    pub(crate) fn definition(&self, idx: usize) -> &DefinitionRecord {
        &self.definitions[idx]
    }
}

/// Returns the embedded file-definition package bundled with the DAL crate.
///
/// # Errors
///
/// Returns an error when the embedded `filedefs.dat` asset cannot be decoded.
pub fn bundled_definition_package() -> Result<&'static DefinitionPackage, StorageError> {
    debug!(target: "dhara_storage::definitions", "loading bundled definition package");
    dal::bundled_definition_package().map_err(|err| StorageError::DefinitionsLoad {
        message: err.to_string(),
    })
}

/// Decodes an in-memory `filedefs.dat` blob.
///
/// # Errors
///
/// Returns an error when the package cannot be parsed, validated, or deserialized.
pub fn decode_definition_package(bytes: &[u8]) -> Result<DefinitionPackage, StorageError> {
    info!(
        target: "dhara_storage::definitions",
        byte_len = bytes.len(),
        "decoding FlatBuffers definition package"
    );
    dal::decode_definition_package(bytes).map_err(|err| StorageError::DefinitionsLoad {
        message: err.to_string(),
    })
}

pub(crate) fn database() -> Result<&'static DefinitionDatabase, StorageError> {
    DATABASE
        .as_ref()
        .map_err(|message| StorageError::DefinitionsLoad {
            message: message.clone(),
        })
}

#[cfg(test)]
mod tests {
    use dhara_storage_dal::encode_definition_package;

    use super::{
        DefinitionPackage, bundled_definition_package, database, decode_definition_package,
    };

    #[test]
    fn bundled_dat_loads_successfully() {
        let package =
            bundled_definition_package().expect("bundled definitions package should deserialize");
        assert!(!package.definitions.is_empty());
    }

    #[test]
    fn png_header_returns_png_candidate() {
        let db = database().expect("bundled definitions package should deserialize");
        let header = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

        let candidates = db
            .candidate_indices(&header)
            .into_iter()
            .map(|idx| db.definition(idx))
            .collect::<Vec<_>>();

        assert!(!candidates.is_empty());
        assert!(candidates.iter().any(|definition| {
            definition
                .extensions
                .iter()
                .any(|ext| ext.eq_ignore_ascii_case("png") || ext.eq_ignore_ascii_case(".png"))
        }));
    }

    #[test]
    fn builder_style_package_roundtrip_is_semantically_stable() {
        let package =
            bundled_definition_package().expect("bundled definitions package should deserialize");
        let bytes = encode_definition_package(package);
        let decoded = decode_definition_package(&bytes).expect("encoded package should decode");

        assert_eq!(&decoded, package);
    }

    #[test]
    fn legacy_plain_bytes_are_rejected() {
        let error = DefinitionPackage {
            package_version: "test".to_owned(),
            source_version: "test".to_owned(),
            package_revision: 1,
            tags: 0,
            definitions: Vec::new(),
        };
        let plain = format!("{error:?}");

        let error =
            decode_definition_package(plain.as_bytes()).expect_err("plain bytes should fail");

        assert!(error.to_string().contains("FDEF"));
    }
}
