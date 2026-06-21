use std::collections::HashSet;

use once_cell::sync::Lazy;

use super::{
    ParsedTridDefinition,
    mime::{MimeConfidence, MimeResolution},
};

const LEVEL1: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/extensions/level1.txt"
));
const LEVEL2: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/extensions/level2.txt"
));
const LEVEL3: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/extensions/level3.txt"
));
const LEVEL4: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/extensions/level4.txt"
));
const LEVEL5: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/extensions/level5.txt"
));

static EXTENSION_SEEDS: Lazy<ExtensionSeeds> = Lazy::new(ExtensionSeeds::load);

#[derive(Debug)]
pub(crate) struct ExtensionSeeds {
    levels: [HashSet<String>; 5],
}

impl ExtensionSeeds {
    fn load() -> Self {
        Self {
            levels: [
                load_extension_set(LEVEL1),
                load_extension_set(LEVEL2),
                load_extension_set(LEVEL3),
                load_extension_set(LEVEL4),
                load_extension_set(LEVEL5),
            ],
        }
    }

    pub(crate) fn best_level(&self, extensions: &[String]) -> Option<u8> {
        for (index, set) in self.levels.iter().enumerate() {
            if extensions.iter().any(|extension| set.contains(extension)) {
                return Some((index + 1) as u8);
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SluiceCandidate {
    pub(crate) definition: ParsedTridDefinition,
    pub(crate) canonical_mime: String,
    pub(crate) level: u8,
    pub(crate) score: i32,
}

impl SluiceCandidate {
    pub(crate) fn from_definition(
        definition: ParsedTridDefinition,
        level: u8,
        resolution: &MimeResolution,
    ) -> Self {
        let score = score_definition(&definition, level, resolution.confidence);
        Self {
            definition,
            canonical_mime: resolution.canonical.clone(),
            level,
            score,
        }
    }
}

pub(crate) fn extension_seeds() -> &'static ExtensionSeeds {
    &EXTENSION_SEEDS
}

fn load_extension_set(contents: &str) -> HashSet<String> {
    contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.to_ascii_lowercase())
        .collect()
}

fn score_definition(
    definition: &ParsedTridDefinition,
    level: u8,
    confidence: MimeConfidence,
) -> i32 {
    let level_score = match level {
        1 => 1_000,
        2 => 820,
        3 => 640,
        4 => 460,
        5 => 280,
        _ => 0,
    };

    let pattern_score = definition
        .signature
        .patterns
        .iter()
        .map(|pattern| if pattern.position == 0 { 120 } else { 45 })
        .sum::<i32>();
    let string_score = (definition.signature.strings.len() as i32 * 12).min(120);
    let file_count_score = (((definition.file_count.max(1) as f32).log2().floor()) as i32) * 20;
    let confidence_score = match confidence {
        MimeConfidence::ExactIana => 250,
        MimeConfidence::ExactCustom => 220,
        MimeConfidence::FuzzyIana => 140,
        MimeConfidence::FuzzyCustom => 120,
    };

    level_score + pattern_score + string_score + file_count_score + confidence_score
}

#[cfg(test)]
mod tests {
    use super::score_definition;
    use crate::builder::trid_xml::{
        ParsedTridDefinition, TridPattern, TridSignature, mime::MimeConfidence,
    };

    #[test]
    fn higher_quality_definitions_score_above_weaker_ones() {
        let strong = ParsedTridDefinition {
            source_version: "2.00".to_string(),
            file_type: "Strong".to_string(),
            extensions: vec!["png".to_string()],
            mime_type: "image/png".to_string(),
            remarks: String::new(),
            signature: TridSignature {
                patterns: vec![
                    TridPattern {
                        position: 0,
                        data: vec![0x89, 0x50, 0x4E, 0x47],
                    },
                    TridPattern {
                        position: 8,
                        data: vec![0x49, 0x48, 0x44, 0x52],
                    },
                ],
                strings: vec![b"IHDR".to_vec()],
            },
            file_count: 500,
        };
        let weak = ParsedTridDefinition {
            source_version: "2.00".to_string(),
            file_type: "Weak".to_string(),
            extensions: vec!["png".to_string()],
            mime_type: "image/png".to_string(),
            remarks: String::new(),
            signature: TridSignature {
                patterns: vec![TridPattern {
                    position: 16,
                    data: vec![0x00],
                }],
                strings: Vec::new(),
            },
            file_count: 2,
        };

        let strong_score = score_definition(&strong, 1, MimeConfidence::ExactIana);
        let weak_score = score_definition(&weak, 1, MimeConfidence::FuzzyIana);

        assert!(strong_score > weak_score);
    }
}
