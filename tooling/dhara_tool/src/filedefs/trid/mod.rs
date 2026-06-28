use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use rayon::prelude::*;

use dhara_storage_dal::{
    DefinitionPackage, DefinitionRecord, SignatureDefinition, SignaturePattern,
};
use tracing::debug;

use crate::filedefs::BuilderError;
use crate::workspace::next_package_revision_for_build;

mod mime;
mod model;
mod sluice;
mod source;
mod source_manifest;

use mime::mime_catalog;
use sluice::{SluiceCandidate, extension_seeds};
use source_manifest::load_definitions_release;

const VALIDATED_TAGS: u32 = 48;
const TARGET_DEFINITION_COUNT: usize = 5_500;

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

/// Diagnostics produced while transforming TrID XML definitions into a FlatBuffers package.
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

/// The result of building a FlatBuffers package from TrID XML definitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TridBuildOutput {
    /// The transformed definitions package.
    pub package: DefinitionPackage,
    /// Diagnostics describing how the package was produced.
    pub report: TridTransformReport,
}

/// Build a reduced `filedefs.dat` package from a TrID XML source.
///
/// The source may be a single `.xml` definition file, a directory that contains
/// extracted TrID XML definitions, or a `.7z` archive containing the XML tree.
///
/// # Returns
///
/// - `Result<DefinitionPackage, BuilderError>` - A reduced package compatible with the Dhara definition package format.
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

/// Build a reduced `filedefs.dat` package from a TrID XML source while reporting progress.
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

/// Build a reduced `filedefs.dat` package from a TrID XML source and return diagnostics.
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
    debug!(source = %source.display(), "building reduced TrID XML package");
    progress(TridBuildProgress {
        stage: TridBuildStage::LoadSource,
        message: format!("Loading source {}", source.display()),
        current: 0,
        total: None,
        current_item: Some(source.display().to_string()),
        stats: TridBuildStats::default(),
    });
    let parsed = source::load_trid_definitions(source, progress)?;
    let definitions_release = load_definitions_release(source)?;
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

    let mut survivors = if report.total_parsed <= PARALLEL_REDUCE_THRESHOLD {
        reduce_definitions_sequential(parsed, &mut report, progress)
    } else {
        reduce_definitions_parallel(parsed, &mut report)
    };

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
    debug!(
        final_kept = report.final_kept,
        final_trimmed = report.final_trimmed,
        "reduced TrID definitions package ready"
    );

    let tool_version = crate::version();
    let package_revision = next_package_revision_for_build(tool_version)
        .map_err(|message| BuilderError::Package { message })?;

    let package = DefinitionPackage {
        package_version: tool_version.to_owned(),
        definitions_release,
        package_revision,
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
    pub(crate) file_type: String,
    pub(crate) extensions: Vec<String>,
    pub(crate) mime_type: String,
    pub(crate) remarks: String,
    pub(crate) signature: TridSignature,
    pub(crate) file_count: u32,
}

const PARALLEL_REDUCE_THRESHOLD: usize = 8;

enum ReduceOutcome {
    SignatureRejected,
    ExtensionRejected,
    MimeRejected,
    Accepted(SluiceCandidate),
}

fn reduce_definition(
    definition: ParsedTridDefinition,
    catalog: &mime::MimeCatalog,
    seeds: &sluice::ExtensionSeeds,
) -> (ReduceOutcome, bool) {
    if definition.signature.patterns.is_empty() {
        return (ReduceOutcome::SignatureRejected, false);
    }

    let Some(level) = seeds.best_level(&definition.extensions) else {
        return (ReduceOutcome::ExtensionRejected, false);
    };

    let raw_mime = definition.mime_type.clone();
    let mut mime_cache = HashMap::new();
    let Some(mime) = catalog.canonicalize(&raw_mime, &mut mime_cache) else {
        return (ReduceOutcome::MimeRejected, false);
    };

    let mime_corrected = raw_mime.trim().to_ascii_lowercase() != mime.canonical;
    (
        ReduceOutcome::Accepted(SluiceCandidate::from_definition(definition, level, &mime)),
        mime_corrected,
    )
}

fn reduce_definitions_sequential(
    parsed: Vec<ParsedTridDefinition>,
    report: &mut TridTransformReport,
    progress: &mut dyn FnMut(TridBuildProgress),
) -> Vec<SluiceCandidate> {
    let mut mime_cache = HashMap::new();
    let mut survivors = Vec::new();
    let catalog = mime_catalog();
    let seeds = extension_seeds();
    let total = report.total_parsed;

    for (index, definition) in parsed.into_iter().enumerate() {
        let (outcome, mime_corrected) =
            reduce_definition_with_cache(definition, catalog, seeds, &mut mime_cache);
        apply_reduce_outcome(
            outcome,
            mime_corrected,
            index + 1,
            total,
            report,
            &mut survivors,
            progress,
            true,
        );
    }

    survivors
}

fn reduce_definition_with_cache(
    definition: ParsedTridDefinition,
    catalog: &mime::MimeCatalog,
    seeds: &sluice::ExtensionSeeds,
    mime_cache: &mut HashMap<String, Option<mime::MimeResolution>>,
) -> (ReduceOutcome, bool) {
    if definition.signature.patterns.is_empty() {
        return (ReduceOutcome::SignatureRejected, false);
    }

    let Some(level) = seeds.best_level(&definition.extensions) else {
        return (ReduceOutcome::ExtensionRejected, false);
    };

    let raw_mime = definition.mime_type.clone();
    let Some(mime) = catalog.canonicalize(&raw_mime, mime_cache) else {
        return (ReduceOutcome::MimeRejected, false);
    };

    let mime_corrected = raw_mime.trim().to_ascii_lowercase() != mime.canonical;
    (
        ReduceOutcome::Accepted(SluiceCandidate::from_definition(definition, level, &mime)),
        mime_corrected,
    )
}

fn reduce_definitions_parallel(
    parsed: Vec<ParsedTridDefinition>,
    report: &mut TridTransformReport,
) -> Vec<SluiceCandidate> {
    let catalog = mime_catalog();
    let seeds = extension_seeds();
    let total = report.total_parsed;
    let completed = AtomicUsize::new(0);
    let signature_rejected = AtomicUsize::new(0);
    let extension_rejected = AtomicUsize::new(0);
    let mime_rejected = AtomicUsize::new(0);
    let mime_corrected = AtomicUsize::new(0);
    let accepted_count = AtomicUsize::new(0);

    let outcomes: Vec<(ReduceOutcome, bool)> = parsed
        .into_par_iter()
        .map(|definition| {
            let file_type = definition.file_type.clone();
            let (outcome, corrected) = reduce_definition(definition, catalog, seeds);
            let done = completed.fetch_add(1, Ordering::Relaxed) + 1;

            match &outcome {
                ReduceOutcome::SignatureRejected => {
                    signature_rejected.fetch_add(1, Ordering::Relaxed);
                    emit_parallel_reduce_progress(
                        done,
                        total,
                        Some(&file_type),
                        "Rejecting definition without patterns",
                        &ParallelReduceCounters {
                            signature_rejected: signature_rejected.load(Ordering::Relaxed),
                            extension_rejected: extension_rejected.load(Ordering::Relaxed),
                            mime_rejected: mime_rejected.load(Ordering::Relaxed),
                            mime_corrected: mime_corrected.load(Ordering::Relaxed),
                            accepted_count: accepted_count.load(Ordering::Relaxed),
                        },
                    );
                }
                ReduceOutcome::ExtensionRejected => {
                    extension_rejected.fetch_add(1, Ordering::Relaxed);
                    emit_parallel_reduce_progress(
                        done,
                        total,
                        Some(&file_type),
                        "Rejecting definition by extension floodgate",
                        &ParallelReduceCounters {
                            signature_rejected: signature_rejected.load(Ordering::Relaxed),
                            extension_rejected: extension_rejected.load(Ordering::Relaxed),
                            mime_rejected: mime_rejected.load(Ordering::Relaxed),
                            mime_corrected: mime_corrected.load(Ordering::Relaxed),
                            accepted_count: accepted_count.load(Ordering::Relaxed),
                        },
                    );
                }
                ReduceOutcome::MimeRejected => {
                    mime_rejected.fetch_add(1, Ordering::Relaxed);
                    emit_parallel_reduce_progress(
                        done,
                        total,
                        Some(&file_type),
                        "Rejecting definition by MIME validation",
                        &ParallelReduceCounters {
                            signature_rejected: signature_rejected.load(Ordering::Relaxed),
                            extension_rejected: extension_rejected.load(Ordering::Relaxed),
                            mime_rejected: mime_rejected.load(Ordering::Relaxed),
                            mime_corrected: mime_corrected.load(Ordering::Relaxed),
                            accepted_count: accepted_count.load(Ordering::Relaxed),
                        },
                    );
                }
                ReduceOutcome::Accepted(_) => {
                    if corrected {
                        mime_corrected.fetch_add(1, Ordering::Relaxed);
                    }
                    accepted_count.fetch_add(1, Ordering::Relaxed);
                    emit_parallel_reduce_progress(
                        done,
                        total,
                        Some(&file_type),
                        "Accepting validated definition",
                        &ParallelReduceCounters {
                            signature_rejected: signature_rejected.load(Ordering::Relaxed),
                            extension_rejected: extension_rejected.load(Ordering::Relaxed),
                            mime_rejected: mime_rejected.load(Ordering::Relaxed),
                            mime_corrected: mime_corrected.load(Ordering::Relaxed),
                            accepted_count: accepted_count.load(Ordering::Relaxed),
                        },
                    );
                }
            }

            (outcome, corrected)
        })
        .collect();

    report.signature_rejected = signature_rejected.load(Ordering::Relaxed);
    report.extension_rejected = extension_rejected.load(Ordering::Relaxed);
    report.mime_rejected = mime_rejected.load(Ordering::Relaxed);
    report.mime_corrected = mime_corrected.load(Ordering::Relaxed);

    outcomes
        .into_iter()
        .filter_map(|(outcome, _)| match outcome {
            ReduceOutcome::Accepted(candidate) => Some(candidate),
            _ => None,
        })
        .collect()
}

struct ParallelReduceCounters {
    signature_rejected: usize,
    extension_rejected: usize,
    mime_rejected: usize,
    mime_corrected: usize,
    accepted_count: usize,
}

fn emit_parallel_reduce_progress(
    current: usize,
    total: usize,
    current_item: Option<&str>,
    message: &str,
    counters: &ParallelReduceCounters,
) {
    crate::logging::emit_trid_progress(TridBuildProgress {
        stage: TridBuildStage::ReduceDefinitions,
        message: message.to_string(),
        current,
        total: Some(total),
        current_item: current_item.map(str::to_owned),
        stats: TridBuildStats {
            parsed_count: total,
            accepted_count: counters.accepted_count,
            mime_corrected: counters.mime_corrected,
            mime_rejected: counters.mime_rejected,
            extension_rejected: counters.extension_rejected,
            signature_rejected: counters.signature_rejected,
            final_trimmed: 0,
        },
    });
}

#[allow(clippy::too_many_arguments)]
fn apply_reduce_outcome(
    outcome: ReduceOutcome,
    mime_corrected: bool,
    current: usize,
    total: usize,
    report: &mut TridTransformReport,
    survivors: &mut Vec<SluiceCandidate>,
    progress: &mut dyn FnMut(TridBuildProgress),
    emit_item_progress: bool,
) {
    let (message, current_item) = match outcome {
        ReduceOutcome::SignatureRejected => {
            report.signature_rejected += 1;
            ("Rejecting definition without patterns".to_string(), None)
        }
        ReduceOutcome::ExtensionRejected => {
            report.extension_rejected += 1;
            (
                "Rejecting definition by extension floodgate".to_string(),
                None,
            )
        }
        ReduceOutcome::MimeRejected => {
            report.mime_rejected += 1;
            ("Rejecting definition by MIME validation".to_string(), None)
        }
        ReduceOutcome::Accepted(candidate) => {
            if mime_corrected {
                report.mime_corrected += 1;
            }
            let current_item = candidate.definition.file_type.clone();
            survivors.push(candidate);
            (
                "Accepting validated definition".to_string(),
                Some(current_item),
            )
        }
    };

    if emit_item_progress {
        progress(TridBuildProgress {
            stage: TridBuildStage::ReduceDefinitions,
            message,
            current,
            total: Some(total),
            current_item,
            stats: TridBuildStats {
                parsed_count: total,
                accepted_count: survivors.len(),
                mime_corrected: report.mime_corrected,
                mime_rejected: report.mime_rejected,
                extension_rejected: report.extension_rejected,
                signature_rejected: report.signature_rejected,
                final_trimmed: report.final_trimmed,
            },
        });
    }
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
