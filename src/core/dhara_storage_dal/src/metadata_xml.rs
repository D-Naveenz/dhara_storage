use serde::{Deserialize, Serialize};

use crate::error::DefinitionPackageError;
use crate::model::{DEFINITION_PACKAGE_SIGNATURE, DSFD_METADATA_XMLNS, DefinitionPackage};

/// Parsed XML metadata from the DSFD file footer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename = "dsfd")]
pub struct DsfdMetadataXml {
    /// XML namespace for the metadata document.
    #[serde(rename = "@xmlns")]
    pub xmlns: String,
    /// Human-readable package signature.
    pub signature: String,
    /// `dhara_tool` version used to build the package.
    #[serde(rename = "packageVersion")]
    pub package_version: String,
    /// ISO release date of the upstream definitions dataset.
    #[serde(rename = "definitionsRelease")]
    pub definitions_release: String,
    /// Builder sluice revision.
    #[serde(rename = "packageRevision")]
    pub package_revision: u16,
    /// Builder validation flags.
    pub tags: u32,
    /// Number of definitions in the FlatBuffers payload.
    #[serde(rename = "definitionCount")]
    pub definition_count: u32,
}

impl DsfdMetadataXml {
    pub(crate) fn from_package(package: &DefinitionPackage) -> Self {
        Self {
            xmlns: DSFD_METADATA_XMLNS.to_owned(),
            signature: DEFINITION_PACKAGE_SIGNATURE.to_owned(),
            package_version: package.package_version.clone(),
            definitions_release: package.definitions_release.clone(),
            package_revision: package.package_revision,
            tags: package.tags,
            definition_count: package.definitions.len() as u32,
        }
    }

    pub(crate) fn apply_to_package(
        self,
        package: &mut DefinitionPackage,
    ) -> Result<(), DefinitionPackageError> {
        if self.xmlns != DSFD_METADATA_XMLNS {
            return Err(DefinitionPackageError::InvalidMetadata {
                message: format!("unexpected xmlns '{}'", self.xmlns),
            });
        }
        if self.signature != DEFINITION_PACKAGE_SIGNATURE {
            return Err(DefinitionPackageError::InvalidMetadata {
                message: "metadata signature does not match the expected DSFD signature".to_owned(),
            });
        }
        if self.package_revision != package.package_revision {
            return Err(DefinitionPackageError::InvalidMetadata {
                message: "metadata packageRevision does not match the FlatBuffers payload"
                    .to_owned(),
            });
        }
        if self.tags != package.tags {
            return Err(DefinitionPackageError::InvalidMetadata {
                message: "metadata tags do not match the FlatBuffers payload".to_owned(),
            });
        }
        if self.definition_count as usize != package.definitions.len() {
            return Err(DefinitionPackageError::InvalidMetadata {
                message: format!(
                    "metadata definitionCount {} does not match payload definition count {}",
                    self.definition_count,
                    package.definitions.len()
                ),
            });
        }
        package.package_version = self.package_version;
        package.definitions_release = self.definitions_release;
        Ok(())
    }
}

/// Serialize package metadata into a compact single-line XML document.
pub fn serialize_metadata(package: &DefinitionPackage) -> Vec<u8> {
    let metadata = DsfdMetadataXml::from_package(package);
    let mut buffer = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    quick_xml::se::to_writer(&mut buffer, &metadata).expect("metadata should serialize");
    buffer.into_bytes()
}

/// Parse XML metadata from the file footer and merge it into `package`.
pub fn deserialize_metadata(
    xml: &[u8],
    package: &mut DefinitionPackage,
) -> Result<(), DefinitionPackageError> {
    let metadata: DsfdMetadataXml = quick_xml::de::from_reader(xml).map_err(|error| {
        DefinitionPackageError::InvalidMetadataXml {
            message: error.to_string(),
        }
    })?;
    metadata.apply_to_package(package)
}

#[cfg(test)]
mod tests {
    use super::{deserialize_metadata, serialize_metadata};
    use crate::model::{DefinitionPackage, DefinitionRecord};

    fn sample_package() -> DefinitionPackage {
        DefinitionPackage {
            package_version: "0.6.0".to_owned(),
            definitions_release: "2026-06-24".to_owned(),
            package_revision: 1,
            tags: 48,
            definitions: vec![DefinitionRecord::default()],
        }
    }

    #[test]
    fn metadata_xml_roundtrip() {
        let package = sample_package();
        let xml = serialize_metadata(&package);
        let mut decoded = DefinitionPackage {
            package_version: String::new(),
            definitions_release: String::new(),
            package_revision: package.package_revision,
            tags: package.tags,
            definitions: package.definitions.clone(),
        };
        deserialize_metadata(&xml, &mut decoded).expect("metadata should parse");
        assert_eq!(decoded, package);
        assert!(xml.starts_with(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(!xml.windows(2).any(|window| window == b"\n\n"));
    }
}
