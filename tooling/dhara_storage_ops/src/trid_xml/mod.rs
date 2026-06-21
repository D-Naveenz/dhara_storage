use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use dhara_storage::{DefinitionPackage, DefinitionRecord, SignatureDefinition, SignaturePattern};
use tracing::{debug, info};

use crate::builder::BuilderError;

mod mime;
mod model;
mod sluice;
mod source;

use mime::mime_catalog;
use sluice::{SluiceCandidate, extension_seeds};

const VALIDATED_TAGS: u32 = 48;
const TARGET_DEFINITION_COUNT: usize = 5_500;
const PACKAGE_REVISION: u16 = 1;

/// Progress stages emitted while transforming TrID XML into a reduced package.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TridBuildStage {
    /// Reading the source path and deciding how to load it.
    LoadSource,
    /// Extracting a `.7z` archive into a temporary directory.
    ExtractArchive,
    /// Parsing XML definitions from the source.
    ParseDefinitions,
    /// Validating and correcting MIME types and extension eligibility.
    ReduceDefinitions,
    /// Ordering and trimming the reduced definition set.
    FinalizePackage,
}

/// A progress update emitted while building a reduced TrID package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TridBuildProgress {
    /// The current transformation stage.
    pub stage: TridBuildStage,
    /// Human-readable description of the active work.
    pub message: String,
    /// Completed units within the current stage.
    pub current: usize,
    /// Total units expected for the current stage when known.
    pub total: Option<usize>,
    /// The file or definition currently being processed when available.
    pub current_item: Option<String>,
    /// Live counters collected while the build is running.
    pub stats: TridBuildStats,
}

/// Live counters exposed during TrID package building.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TridBuildStats {
    /// Number of parsed XML definitions discovered so far.
    pub parsed_count: usize,
    /// Number of accepted definitions that survived validation.
    pub accepted_count: usize,
    /// Number of MIME values corrected to a canonical value.
    pub mime_corrected: usize,
    /// Number of definitions rejected because their MIME could not be recognized.
    pub mime_rejected: usize,
    /// Number of definitions rejected because their extensions were filtered out.
    pub extension_rejected: usize,
    /// Number of definitions rejected because no signature patterns were available.
    pub signature_rejected: usize,
    /// Number of definitions trimmed after ranking.
    pub final_trimmed: usize,
}

/// Diagnostics produced while transforming TrID XML definitions into a `dhbin`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TridTransformReport {
    /// Total parsed TrID definitions before validation.
    pub total_parsed: usize,
    /// Definitions whose MIME type was repaired to a canonical value.
    pub mime_corrected: usize,
    /// Definitions rejected because the MIME type could not be recognized.
    pub mime_rejected: usize,
    /// Definitions rejected because no seeded common extension survived.
    pub extension_rejected: usize,
    /// Definitions rejected because they had no usable signature patterns.
    pub signature_rejected: usize,
    /// Definitions trimmed after ranking to keep the reduced package focused.
    pub final_trimmed: usize,
    /// Final number of definitions emitted into the package.
    pub final_kept: usize,
}

/// The result of building a `dhbin` from TrID XML definitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TridBuildOutput {
    /// The transformed definitions package.
    pub package: DefinitionPackage,
    /// Diagnostics describing how the package was produced.
    pub report: TridTransformReport,
}

/// Build a reduced `filedefs.dhbin` package from a TrID XML source.
///
/// The source may be a single `.xml` definition file, a directory that contains
/// extracted TrID XML definitions, or a `.7z` archive containing the XML tree.
///
/// # Returns
///
/// - `Result<DefinitionPackage, BuilderError>` - A reduced package compatible with `dhara_storage`.
///
/// # Errors
///
/// Returns an error when the source cannot be opened, extracted, parsed, or
/// transformed into a valid package.
///
/// # Examples
///
/// ```ignore
/// # // Internal CLI helper; exercised through the binary integration tests.
/// ```
#[cfg_attr(not(test), allow(dead_code))]
pub fn build_trid_xml_package(source: impl AsRef<Path>) -> Result<DefinitionPackage, BuilderError> {
    Ok(build_trid_xml_package_with_report(source)?.package)
}

/// Build a reduced `filedefs.dhbin` package from a TrID XML source while reporting progress.
///
/// # Returns
///
/// - `Result<TridBuildOutput, BuilderError>` - The reduced package and transformation report.
///
/// # Errors
///
/// Returns an error when the source cannot be transformed successfully.
pub fn build_trid_xml_package_with_progress<F>(
    source: impl AsRef<Path>,
    mut progress: F,
) -> Result<TridBuildOutput, BuilderError>
where
    F: FnMut(TridBuildProgress),
{
    build_trid_xml_package_with_report_internal(source.as_ref(), &mut progress)
}

/// Build a reduced `filedefs.dhbin` package from a TrID XML source and return diagnostics.
///
/// # Returns
///
/// - `Result<TridBuildOutput, BuilderError>` - The reduced package and transformation report.
///
/// # Errors
///
/// Returns an error when the source cannot be transformed successfully.
#[cfg_attr(not(test), allow(dead_code))]
pub fn build_trid_xml_package_with_report(
    source: impl AsRef<Path>,
) -> Result<TridBuildOutput, BuilderError> {
    build_trid_xml_package_with_report_internal(source.as_ref(), &mut |_| {})
}

fn build_trid_xml_package_with_report_internal(
    source: &Path,
    progress: &mut dyn FnMut(TridBuildProgress),
) -> Result<TridBuildOutput, BuilderError> {
    info!(source = %source.display(), "building reduced TrID XML package");
    progress(TridBuildProgress {
        stage: TridBuildStage::LoadSource,
        message: format!("Loading source {}", source.display()),
        current: 0,
        total: None,
        current_item: Some(source.display().to_string()),
        stats: TridBuildStats::default(),
    });
    let parsed = source::load_trid_definitions(source, progress)?;
    let source_version = determine_source_version(&parsed)?;
    let mut report = TridTransformReport {
        total_parsed: parsed.len(),
        ..TridTransformReport::default()
    };
    progress(TridBuildProgress {
        stage: TridBuildStage::ReduceDefinitions,
        message: "Reducing validated definitions".to_string(),
        current: 0,
        total: Some(report.total_parsed),
        current_item: None,
        stats: TridBuildStats {
            parsed_count: report.total_parsed,
            ..TridBuildStats::default()
        },
    });

    let mut mime_cache = HashMap::new();
    let mut survivors = Vec::new();
    let catalog = mime_catalog();
    let seeds = extension_seeds();

    for (index, definition) in parsed.into_iter().enumerate() {
        if definition.signature.patterns.is_empty() {
            report.signature_rejected += 1;
            progress(TridBuildProgress {
                stage: TridBuildStage::ReduceDefinitions,
                message: "Rejecting definition without patterns".to_string(),
                current: index + 1,
                total: Some(report.total_parsed),
                current_item: Some(definition.file_type.clone()),
                stats: TridBuildStats {
                    parsed_count: report.total_parsed,
                    accepted_count: survivors.len(),
                    mime_corrected: report.mime_corrected,
                    mime_rejected: report.mime_rejected,
                    extension_rejected: report.extension_rejected,
                    signature_rejected: report.signature_rejected,
                    final_trimmed: report.final_trimmed,
                },
            });
            continue;
        }

        let Some(level) = seeds.best_level(&definition.extensions) else {
            report.extension_rejected += 1;
            progress(TridBuildProgress {
                stage: TridBuildStage::ReduceDefinitions,
                message: "Rejecting definition by extension floodgate".to_string(),
                current: index + 1,
                total: Some(report.total_parsed),
                current_item: Some(definition.file_type.clone()),
                stats: TridBuildStats {
                    parsed_count: report.total_parsed,
                    accepted_count: survivors.len(),
                    mime_corrected: report.mime_corrected,
                    mime_rejected: report.mime_rejected,
                    extension_rejected: report.extension_rejected,
                    signature_rejected: report.signature_rejected,
                    final_trimmed: report.final_trimmed,
                },
            });
            continue;
        };

        let raw_mime = definition.mime_type.clone();
        let Some(mime) = catalog.canonicalize(&raw_mime, &mut mime_cache) else {
            report.mime_rejected += 1;
            progress(TridBuildProgress {
                stage: TridBuildStage::ReduceDefinitions,
                message: "Rejecting definition by MIME validation".to_string(),
                current: index + 1,
                total: Some(report.total_parsed),
                current_item: Some(definition.file_type.clone()),
                stats: TridBuildStats {
                    parsed_count: report.total_parsed,
                    accepted_count: survivors.len(),
                    mime_corrected: report.mime_corrected,
                    mime_rejected: report.mime_rejected,
                    extension_rejected: report.extension_rejected,
                    signature_rejected: report.signature_rejected,
                    final_trimmed: report.final_trimmed,
                },
            });
            continue;
        };

        if raw_mime.trim().to_ascii_lowercase() != mime.canonical {
            report.mime_corrected += 1;
        }

        let current_item = definition.file_type.clone();
        survivors.push(SluiceCandidate::from_definition(definition, level, &mime));
        progress(TridBuildProgress {
            stage: TridBuildStage::ReduceDefinitions,
            message: "Accepting validated definition".to_string(),
            current: index + 1,
            total: Some(report.total_parsed),
            current_item: Some(current_item),
            stats: TridBuildStats {
                parsed_count: report.total_parsed,
                accepted_count: survivors.len(),
                mime_corrected: report.mime_corrected,
                mime_rejected: report.mime_rejected,
                extension_rejected: report.extension_rejected,
                signature_rejected: report.signature_rejected,
                final_trimmed: report.final_trimmed,
            },
        });
    }

    debug!(
        total_parsed = report.total_parsed,
        mime_corrected = report.mime_corrected,
        mime_rejected = report.mime_rejected,
        extension_rejected = report.extension_rejected,
        signature_rejected = report.signature_rejected,
        survivors = survivors.len(),
        "completed TrID validation and reduction pass"
    );

    survivors.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.level.cmp(&right.level))
            .then_with(|| left.definition.file_type.cmp(&right.definition.file_type))
            .then_with(|| left.definition.mime_type.cmp(&right.definition.mime_type))
            .then_with(|| left.definition.extensions.cmp(&right.definition.extensions))
            .then_with(|| left.definition.remarks.cmp(&right.definition.remarks))
    });

    if survivors.len() > TARGET_DEFINITION_COUNT {
        report.final_trimmed = survivors.len() - TARGET_DEFINITION_COUNT;
        survivors.truncate(TARGET_DEFINITION_COUNT);
    }
    report.final_kept = survivors.len();
    progress(TridBuildProgress {
        stage: TridBuildStage::FinalizePackage,
        message: "Finalizing reduced package".to_string(),
        current: report.final_kept,
        total: Some(report.final_kept),
        current_item: None,
        stats: TridBuildStats {
            parsed_count: report.total_parsed,
            accepted_count: report.final_kept,
            mime_corrected: report.mime_corrected,
            mime_rejected: report.mime_rejected,
            extension_rejected: report.extension_rejected,
            signature_rejected: report.signature_rejected,
            final_trimmed: report.final_trimmed,
        },
    });
    info!(
        final_kept = report.final_kept,
        final_trimmed = report.final_trimmed,
        "reduced TrID definitions package ready"
    );

    let package = DefinitionPackage {
        package_version: format!("trid-{source_version}+dhbn.{PACKAGE_REVISION}"),
        source_version,
        package_revision: PACKAGE_REVISION,
        tags: VALIDATED_TAGS,
        definitions: survivors
            .into_iter()
            .map(candidate_to_record)
            .collect::<Vec<_>>(),
    };

    Ok(TridBuildOutput { package, report })
}

/// Inspect a TrID XML source without writing a package file.
///
/// # Returns
///
/// - `Result<TridTransformReport, BuilderError>` - Diagnostics for the transformed package.
///
/// # Errors
///
/// Returns an error when the source cannot be parsed into a package.
///
/// # Examples
///
/// ```ignore
/// # // Internal CLI helper; exercised through the binary integration tests.
/// ```
#[cfg_attr(not(test), allow(dead_code))]
pub fn inspect_trid_xml_source(
    source: impl AsRef<Path>,
) -> Result<TridTransformReport, BuilderError> {
    Ok(build_trid_xml_package_with_report(source)?.report)
}

#[derive(Debug, Clone)]
pub(crate) struct TridPattern {
    pub(crate) position: u16,
    pub(crate) data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub(crate) struct TridSignature {
    pub(crate) patterns: Vec<TridPattern>,
    pub(crate) strings: Vec<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedTridDefinition {
    pub(crate) source_version: String,
    pub(crate) file_type: String,
    pub(crate) extensions: Vec<String>,
    pub(crate) mime_type: String,
    pub(crate) remarks: String,
    pub(crate) signature: TridSignature,
    pub(crate) file_count: u32,
}

fn candidate_to_record(candidate: SluiceCandidate) -> DefinitionRecord {
    DefinitionRecord {
        file_type: candidate.definition.file_type,
        extensions: candidate.definition.extensions,
        mime_type: candidate.canonical_mime,
        remarks: candidate.definition.remarks,
        signature: SignatureDefinition {
            patterns: candidate
                .definition
                .signature
                .patterns
                .into_iter()
                .map(|pattern| SignaturePattern {
                    position: pattern.position,
                    data: pattern.data,
                })
                .collect(),
            strings: candidate.definition.signature.strings,
        },
        priority_level: candidate.score,
    }
}

fn determine_source_version(parsed: &[ParsedTridDefinition]) -> Result<String, BuilderError> {
    let versions = parsed
        .iter()
        .map(|definition| definition.source_version.trim())
        .filter(|version| !version.is_empty())
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>();

    let Some(source_version) = versions
        .iter()
        .max_by(|left, right| version_key(left).cmp(&version_key(right)))
        .cloned()
    else {
        return Err(BuilderError::MissingSourceVersion {
            versions: "<missing>".to_string(),
        });
    };

    if versions.len() > 1 {
        debug!(
            selected = %source_version,
            discovered = ?versions,
            "multiple TrID source versions were present; selected the highest version"
        );
    }

    Ok(source_version)
}

fn version_key(version: &str) -> Vec<u32> {
    version
        .split('.')
        .map(|segment| segment.parse::<u32>().unwrap_or(0))
        .collect()
}
