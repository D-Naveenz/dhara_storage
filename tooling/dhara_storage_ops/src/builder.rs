use std::fs;
use std::path::{Path, PathBuf};

use dhara_dhbin::{
    CompressionKind, DhbinReadOptions, DhbinReader, DhbinWriteOptions, DhbinWriter, IntegrityKind,
    PackagePurpose,
};
use dhara_storage::{
    DEFINITION_PACKAGE_ID, DefinitionPackage, DefinitionRecord, bundled_definition_package,
    decode_definition_package_payload,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info};

#[path = "trid_xml/mod.rs"]
mod trid_xml;

pub use trid_xml::{
    TridBuildProgress, TridBuildStage, TridTransformReport, build_trid_xml_package_with_progress,
};

#[derive(Debug, Error)]
pub enum BuilderError {
    #[error("failed to {operation} '{path}': {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("package error: {message}")]
    Package { message: String },

    #[error("failed to parse TrID XML '{path}': {message}")]
    Xml { path: PathBuf, message: String },

    #[error("invalid hex sequence '{value}' in '{path}'")]
    InvalidHex { path: PathBuf, value: String },

    #[error("unsupported TrID source '{path}': expected a .7z archive, .xml file, or directory")]
    UnsupportedSource { path: PathBuf },

    #[error("archive tool '{tool}' is not available on PATH")]
    ArchiveToolUnavailable { tool: &'static str },

    #[error("failed to {operation} archive '{path}': {message}")]
    ArchiveCommand {
        operation: &'static str,
        path: PathBuf,
        message: String,
    },

    #[error("failed to determine a usable TrID source version from: {versions}")]
    MissingSourceVersion { versions: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageSummary {
    pub(crate) package_id: String,
    pub(crate) purpose: PackagePurpose,
    pub(crate) compression: CompressionKind,
    pub(crate) integrity: IntegrityKind,
    pub(crate) package_version: String,
    pub(crate) source_version: String,
    pub(crate) package_revision: u16,
    pub(crate) checksum_verified: bool,
    pub(crate) tags: u32,
    pub(crate) definition_count: usize,
}

impl PackageSummary {
    pub(crate) fn from_loaded(loaded: &LoadedPackage) -> Self {
        Self {
            package_id: loaded.package_id_string(),
            purpose: loaded.purpose,
            compression: loaded.compression,
            integrity: loaded.integrity,
            package_version: loaded.metadata.package_version.clone(),
            source_version: loaded.metadata.source_version.clone(),
            package_revision: loaded.metadata.package_revision,
            checksum_verified: loaded.checksum_verified,
            tags: loaded.package.tags,
            definition_count: loaded.package.definitions.len(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedPackage {
    pub(crate) package: DefinitionPackage,
    pub(crate) package_id: [u8; 4],
    pub(crate) purpose: PackagePurpose,
    pub(crate) compression: CompressionKind,
    pub(crate) integrity: IntegrityKind,
    pub(crate) metadata: FiledefsPackageMetadata,
    pub(crate) checksum_verified: bool,
}

impl LoadedPackage {
    fn package_id_string(&self) -> String {
        String::from_utf8_lossy(&self.package_id).into_owned()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncEmbeddedStatus {
    Skipped,
    UpToDate,
    Updated,
    NeedsUpdate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncEmbeddedOutcome {
    pub(crate) input: PathBuf,
    pub(crate) output: PathBuf,
    pub(crate) status: SyncEmbeddedStatus,
    pub(crate) package_version: Option<String>,
    pub(crate) detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FiledefsPackageMetadata {
    package_version: String,
    source_version: String,
    package_revision: u16,
}

#[derive(Debug, Serialize)]
struct RawPackageOut(String, String, u16, (), u32, Vec<DefinitionRecord>);

pub fn load_package(path: impl AsRef<Path>) -> Result<LoadedPackage, BuilderError> {
    let path = path.as_ref();
    info!(path = %path.display(), "loading definitions package");
    let bytes = fs::read(path).map_err(|source| BuilderError::Io {
        operation: "read package",
        path: path.to_path_buf(),
        source,
    })?;

    let package = DhbinReader::read_package(
        &bytes,
        &DhbinReadOptions {
            verify_integrity: true,
            load_metadata: true,
        },
    )
    .map_err(|err| BuilderError::Package {
        message: err.to_string(),
    })?;
    if package.header.package_id != DEFINITION_PACKAGE_ID {
        return Err(BuilderError::Package {
            message: format!(
                "unexpected package identifier '{}'",
                String::from_utf8_lossy(&package.header.package_id)
            ),
        });
    }

    let metadata = package
        .metadata
        .as_deref()
        .ok_or_else(|| BuilderError::Package {
            message: "filedefs package is missing metadata".to_string(),
        })
        .and_then(|bytes| {
            rmp_serde::from_slice::<FiledefsPackageMetadata>(bytes).map_err(|err| {
                BuilderError::Package {
                    message: err.to_string(),
                }
            })
        })?;
    let definitions = decode_definition_package_payload(&package.payload).map_err(|err| {
        BuilderError::Package {
            message: err.to_string(),
        }
    })?;
    if definitions.package_version != metadata.package_version
        || definitions.source_version != metadata.source_version
        || definitions.package_revision != metadata.package_revision
    {
        return Err(BuilderError::Package {
            message: "filedefs payload and metadata version fields do not match".to_string(),
        });
    }

    Ok(LoadedPackage {
        package: definitions,
        package_id: package.header.package_id,
        purpose: package.header.purpose,
        compression: package.header.compression,
        integrity: package.integrity,
        metadata,
        checksum_verified: package.integrity_verified,
    })
}

pub fn load_bundled_package() -> Result<DefinitionPackage, BuilderError> {
    info!("loading bundled runtime definitions package");
    bundled_definition_package()
        .cloned()
        .map_err(|err| BuilderError::Package {
            message: err.to_string(),
        })
}

pub fn write_package(
    package: &DefinitionPackage,
    path: impl AsRef<Path>,
) -> Result<PathBuf, BuilderError> {
    write_package_with_purpose(package, path, PackagePurpose::Standard)
}

pub fn write_package_with_purpose(
    package: &DefinitionPackage,
    path: impl AsRef<Path>,
    purpose: PackagePurpose,
) -> Result<PathBuf, BuilderError> {
    let path = path.as_ref().to_path_buf();
    info!(path = %path.display(), ?purpose, "writing definitions package");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| BuilderError::Io {
            operation: "create output directory for",
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let payload = serialize_definition_package_payload(package)?;
    let metadata = serialize_filedefs_metadata(package)?;
    let bytes = DhbinWriter::write_payload_bytes(
        &payload,
        &DhbinWriteOptions {
            package_id: DEFINITION_PACKAGE_ID,
            purpose,
            compression: CompressionKind::Lz4Frame,
            flags: 0,
            metadata: Some(metadata),
            integrity: IntegrityKind::Sha256,
        },
    )
    .map_err(|err| BuilderError::Package {
        message: err.to_string(),
    })?;
    debug!(
        bytes = bytes.len(),
        definitions = package.definitions.len(),
        ?purpose,
        "encoded definitions package"
    );
    fs::write(&path, bytes).map_err(|source| BuilderError::Io {
        operation: "write package",
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

pub fn normalize_package(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
) -> Result<PathBuf, BuilderError> {
    info!("normalizing definitions package");
    let package = load_package(input)?;
    write_package_with_purpose(&package.package, output, package.purpose)
}

pub fn packages_match(
    left: impl AsRef<Path>,
    right: impl AsRef<Path>,
) -> Result<bool, BuilderError> {
    info!("comparing definitions packages");
    let left = load_package(left)?;
    let right = load_package(right)?;
    Ok(left.package == right.package
        && left.package_id == right.package_id
        && left.purpose == right.purpose
        && left.compression == right.compression
        && left.integrity == right.integrity
        && left.metadata == right.metadata)
}

pub fn inspect_package(path: impl AsRef<Path>) -> Result<PackageSummary, BuilderError> {
    info!("inspecting definitions package");
    let package = load_package(path)?;
    Ok(PackageSummary::from_loaded(&package))
}

pub fn sync_embedded_package(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    check_only: bool,
) -> Result<SyncEmbeddedOutcome, BuilderError> {
    let input = input.as_ref().to_path_buf();
    let output = output.as_ref().to_path_buf();
    info!(
        input = %input.display(),
        output = %output.display(),
        check_only,
        "syncing embedded definitions package"
    );

    if !input.exists() {
        return Ok(SyncEmbeddedOutcome {
            input,
            output,
            status: SyncEmbeddedStatus::Skipped,
            package_version: None,
            detail: "TrID archive was not available; skipped embedded package refresh".to_string(),
        });
    }

    let build = trid_xml::build_trid_xml_package_with_report(&input)?;
    let desired_package = build.package;
    let desired_version = desired_package.package_version.clone();

    let current_state = match load_package(&output) {
        Ok(current) => current_validity_reason(&current, &desired_package),
        Err(error) if output.exists() => {
            Some(format!("current embedded package is invalid: {error}"))
        }
        Err(_) => Some("embedded package is missing".to_string()),
    };

    let Some(reason) = current_state else {
        return Ok(SyncEmbeddedOutcome {
            input,
            output,
            status: SyncEmbeddedStatus::UpToDate,
            package_version: Some(desired_version),
            detail: "embedded package is already current".to_string(),
        });
    };

    if check_only {
        return Ok(SyncEmbeddedOutcome {
            input,
            output,
            status: SyncEmbeddedStatus::NeedsUpdate,
            package_version: Some(desired_version),
            detail: reason,
        });
    }

    write_package_with_purpose(&desired_package, &output, PackagePurpose::Embedded)?;
    Ok(SyncEmbeddedOutcome {
        input,
        output,
        status: SyncEmbeddedStatus::Updated,
        package_version: Some(desired_version),
        detail: reason,
    })
}

fn current_validity_reason(current: &LoadedPackage, desired: &DefinitionPackage) -> Option<String> {
    if current.package_id != DEFINITION_PACKAGE_ID {
        return Some(format!(
            "embedded package id '{}' does not match 'FDEF'",
            current.package_id_string()
        ));
    }
    if current.purpose != PackagePurpose::Embedded {
        return Some("embedded package purpose is not Embedded".to_string());
    }
    if current.metadata.package_version != desired.package_version {
        return Some(format!(
            "package version mismatch: current='{}', desired='{}'",
            current.metadata.package_version, desired.package_version
        ));
    }
    if current.metadata.source_version != desired.source_version {
        return Some(format!(
            "source version mismatch: current='{}', desired='{}'",
            current.metadata.source_version, desired.source_version
        ));
    }
    if current.metadata.package_revision != desired.package_revision {
        return Some(format!(
            "package revision mismatch: current='{}', desired='{}'",
            current.metadata.package_revision, desired.package_revision
        ));
    }
    None
}

fn serialize_definition_package_payload(
    package: &DefinitionPackage,
) -> Result<Vec<u8>, BuilderError> {
    rmp_serde::to_vec(&RawPackageOut(
        package.package_version.clone(),
        package.source_version.clone(),
        package.package_revision,
        (),
        package.tags,
        package.definitions.clone(),
    ))
    .map_err(|err| BuilderError::Package {
        message: err.to_string(),
    })
}

fn serialize_filedefs_metadata(package: &DefinitionPackage) -> Result<Vec<u8>, BuilderError> {
    rmp_serde::to_vec(&FiledefsPackageMetadata {
        package_version: package.package_version.clone(),
        source_version: package.source_version.clone(),
        package_revision: package.package_revision,
    })
    .map_err(|err| BuilderError::Package {
        message: err.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use dhara_dhbin::{CompressionKind, IntegrityKind, PackagePurpose};
    use tempfile::tempdir;

    use super::{
        PackageSummary, SyncEmbeddedStatus, load_bundled_package, normalize_package,
        packages_match, sync_embedded_package, trid_xml, write_package, write_package_with_purpose,
    };

    fn fixtures_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("trid_xml")
    }

    #[test]
    fn bundled_package_has_expected_summary() {
        let package = load_bundled_package().expect("bundled package should load");
        assert!(package.package_version.starts_with("trid-"));
        assert!(!package.definitions.is_empty());
    }

    #[test]
    fn normalize_roundtrip_preserves_semantics() {
        let temp = tempdir().unwrap();
        let original = temp.path().join("original.dhbin");
        let normalized = temp.path().join("normalized.dhbin");
        let package = load_bundled_package().expect("bundled package should load");
        write_package(&package, &original).expect("original package should be written");

        normalize_package(&original, &normalized).expect("normalized package should be written");

        assert!(packages_match(&original, &normalized).expect("packages should compare"));
    }

    #[test]
    fn builds_package_from_fixture_directory() {
        let package = trid_xml::build_trid_xml_package(fixtures_root())
            .expect("fixture directory should build");

        assert_eq!(package.tags, 48);
        assert_eq!(package.source_version, "2.00");
        assert_eq!(package.package_revision, 1);
        assert_eq!(package.package_version, "trid-2.00+dhbn.1");
        assert_eq!(package.definitions.len(), 3);
    }

    #[test]
    fn builds_same_package_from_7z_archive() {
        let temp = tempdir().expect("temporary directory should exist");
        let archive_path = temp.path().join("triddefs_xml.7z");

        let status = Command::new("tar")
            .arg("-a")
            .arg("-cf")
            .arg(&archive_path)
            .arg("-C")
            .arg(fixtures_root())
            .arg("defs")
            .status()
            .expect("tar should be available for archive creation");
        assert!(status.success(), "tar should create a 7z archive");

        let from_directory = trid_xml::build_trid_xml_package(fixtures_root())
            .expect("fixture directory should build");
        let from_archive =
            trid_xml::build_trid_xml_package(&archive_path).expect("fixture archive should build");

        assert_eq!(from_archive, from_directory);
    }

    #[test]
    fn inspects_trid_xml_source_without_writing_package() {
        let report = trid_xml::inspect_trid_xml_source(fixtures_root())
            .expect("fixture directory should be inspectable");

        assert_eq!(report.total_parsed, 3);
        assert_eq!(report.mime_corrected, 0);
        assert_eq!(report.mime_rejected, 0);
        assert_eq!(report.extension_rejected, 0);
        assert_eq!(report.signature_rejected, 0);
        assert_eq!(report.final_trimmed, 0);
        assert_eq!(report.final_kept, 3);
    }

    #[test]
    fn malformed_mime_types_are_corrected() {
        let temp = tempdir().expect("temporary directory should exist");
        let defs_dir = temp.path().join("defs").join("a");
        fs::create_dir_all(&defs_dir).expect("fixture directory should be created");
        fs::write(
            defs_dir.join("broken_pdf.trid.xml"),
            r#"<TrID ver="2.00">
    <Info>
        <FileType>Broken PDF</FileType>
        <Ext>PDF</Ext>
        <Mime>Applicaiton/PDF;</Mime>
        <ExtraInfo>
            <Rem></Rem>
            <RefURL>https://example.com/pdf</RefURL>
        </ExtraInfo>
    </Info>
    <General>
        <FileNum>42</FileNum>
    </General>
    <FrontBlock>
        <Pattern>
            <Bytes>255044462D</Bytes>
            <Pos>0</Pos>
        </Pattern>
    </FrontBlock>
</TrID>"#,
        )
        .expect("fixture XML should be written");

        let build = trid_xml::build_trid_xml_package_with_report(temp.path())
            .expect("fixture should build");
        let definition = build
            .package
            .definitions
            .first()
            .expect("definition should be present");

        assert_eq!(definition.mime_type, "application/pdf");
        assert_eq!(build.report.mime_corrected, 1);
        assert_eq!(build.report.final_kept, 1);
    }

    #[test]
    fn builder_writes_full_package_readable_by_runtime() {
        let temp = tempdir().expect("temporary directory should exist");
        let output_path = temp.path().join("filedefs.dhbin");
        let build = trid_xml::build_trid_xml_package_with_report(fixtures_root())
            .expect("fixture should build");

        write_package(&build.package, &output_path).expect("package should be written");
        let loaded = super::load_package(&output_path).expect("package should load");

        assert_eq!(loaded.package, build.package);
        assert_eq!(loaded.purpose, PackagePurpose::Standard);
        assert_eq!(loaded.compression, CompressionKind::Lz4Frame);
        assert_eq!(loaded.integrity, IntegrityKind::Sha256);
        assert!(loaded.checksum_verified);
    }

    #[test]
    fn builder_writes_fast_embedded_packages() {
        let temp = tempdir().expect("temporary directory should exist");
        let output_path = temp.path().join("filedefs.dhbin");
        let build = trid_xml::build_trid_xml_package_with_report(fixtures_root())
            .expect("fixture should build");

        write_package_with_purpose(&build.package, &output_path, PackagePurpose::Embedded)
            .expect("embedded package should be written");
        let loaded = super::load_package(&output_path).expect("package should load");
        assert_eq!(loaded.package, build.package);
        assert_eq!(loaded.purpose, PackagePurpose::Embedded);
    }

    #[test]
    fn inspect_package_reports_v2_metadata() {
        let temp = tempdir().expect("temporary directory should exist");
        let output_path = temp.path().join("filedefs.dhbin");
        let build = trid_xml::build_trid_xml_package_with_report(fixtures_root())
            .expect("fixture should build");
        write_package(&build.package, &output_path).expect("package should be written");

        let summary = super::inspect_package(&output_path).expect("summary should load");
        assert_eq!(
            summary,
            PackageSummary {
                package_id: "FDEF".to_string(),
                purpose: PackagePurpose::Standard,
                compression: CompressionKind::Lz4Frame,
                integrity: IntegrityKind::Sha256,
                package_version: "trid-2.00+dhbn.1".to_string(),
                source_version: "2.00".to_string(),
                package_revision: 1,
                checksum_verified: true,
                tags: 48,
                definition_count: 3,
            }
        );
    }

    #[test]
    fn sync_embedded_skips_when_archive_is_missing() {
        let temp = tempdir().expect("temporary directory should exist");
        let outcome = sync_embedded_package(
            temp.path().join("missing.7z"),
            temp.path().join("filedefs.dhbin"),
            false,
        )
        .expect("sync should succeed");

        assert_eq!(outcome.status, SyncEmbeddedStatus::Skipped);
    }

    #[test]
    fn sync_embedded_updates_when_target_is_stale() {
        let temp = tempdir().expect("temporary directory should exist");
        let archive_path = temp.path().join("triddefs_xml.7z");
        let output_path = temp.path().join("filedefs.dhbin");

        let status = Command::new("tar")
            .arg("-a")
            .arg("-cf")
            .arg(&archive_path)
            .arg("-C")
            .arg(fixtures_root())
            .arg("defs")
            .status()
            .expect("tar should create the fixture archive");
        assert!(status.success(), "tar should create a 7z archive");

        let package = load_bundled_package().expect("bundled package should load");
        write_package(&package, &output_path).expect("stale package should be written");

        let outcome =
            sync_embedded_package(&archive_path, &output_path, false).expect("sync should work");
        assert_eq!(outcome.status, SyncEmbeddedStatus::Updated);

        let loaded = super::load_package(&output_path).expect("updated package should load");
        assert_eq!(loaded.purpose, PackagePurpose::Embedded);
        assert_eq!(loaded.metadata.package_version, "trid-2.00+dhbn.1");
    }

    #[test]
    fn sync_embedded_check_reports_when_update_is_needed() {
        let temp = tempdir().expect("temporary directory should exist");
        let archive_path = temp.path().join("triddefs_xml.7z");
        let output_path = temp.path().join("filedefs.dhbin");

        let status = Command::new("tar")
            .arg("-a")
            .arg("-cf")
            .arg(&archive_path)
            .arg("-C")
            .arg(fixtures_root())
            .arg("defs")
            .status()
            .expect("tar should create the fixture archive");
        assert!(status.success(), "tar should create a 7z archive");

        let output =
            sync_embedded_package(&archive_path, &output_path, true).expect("check should run");
        assert_eq!(output.status, SyncEmbeddedStatus::NeedsUpdate);
    }

    #[test]
    #[ignore = "slow smoke test over the real TrID XML archive"]
    fn full_archive_build_stays_within_target_range() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root should exist")
            .join("temp")
            .join("trid-defs")
            .join("triddefs_xml.7z");
        assert!(source.exists(), "real TrID archive should be available");

        let build = trid_xml::build_trid_xml_package_with_report(&source)
            .expect("real archive should be inspectable");
        let report = &build.report;

        assert!(
            report.final_kept >= 4_500,
            "final_kept={}",
            report.final_kept
        );
        assert!(
            report.final_kept <= 6_500,
            "final_kept={}",
            report.final_kept
        );
        assert!(build.package.definitions.iter().all(|definition| {
            !definition.mime_type.is_empty()
                && !definition.extensions.is_empty()
                && !definition.signature.patterns.is_empty()
        }));
    }
}
