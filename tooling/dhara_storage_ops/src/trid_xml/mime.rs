use std::collections::{HashMap, HashSet};

use once_cell::sync::Lazy;

const IANA_MEDIA_TYPES: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/mime/iana_media_types.txt"
));
const CUSTOM_MEDIA_TYPES: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/mime/custom_media_types.txt"
));
const FUZZY_THRESHOLD: f32 = 0.70;

static MIME_CATALOG: Lazy<MimeCatalog> = Lazy::new(MimeCatalog::load);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MimeConfidence {
    ExactIana,
    ExactCustom,
    FuzzyIana,
    FuzzyCustom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MimeResolution {
    pub(crate) canonical: String,
    pub(crate) confidence: MimeConfidence,
}

#[derive(Debug)]
pub(crate) struct MimeCatalog {
    iana: HashSet<String>,
    custom: HashSet<String>,
    combined: Vec<String>,
}

impl MimeCatalog {
    fn load() -> Self {
        let iana = load_mime_list(IANA_MEDIA_TYPES);
        let custom = load_mime_list(CUSTOM_MEDIA_TYPES);
        let combined = {
            let mut values = iana.union(&custom).cloned().collect::<Vec<_>>();
            values.sort();
            values
        };

        Self {
            iana,
            custom,
            combined,
        }
    }

    pub(crate) fn canonicalize(
        &self,
        mime_type: &str,
        cache: &mut HashMap<String, Option<MimeResolution>>,
    ) -> Option<MimeResolution> {
        if let Some(cached) = cache.get(mime_type) {
            return cached.clone();
        }

        let resolved = self.canonicalize_uncached(mime_type);
        cache.insert(mime_type.to_string(), resolved.clone());
        resolved
    }

    fn canonicalize_uncached(&self, mime_type: &str) -> Option<MimeResolution> {
        let cleaned = clean_basic_mime_type(mime_type);
        if cleaned.is_empty() {
            return None;
        }

        if self.iana.contains(&cleaned) {
            return Some(MimeResolution {
                canonical: cleaned,
                confidence: MimeConfidence::ExactIana,
            });
        }

        if self.custom.contains(&cleaned) {
            return Some(MimeResolution {
                canonical: cleaned,
                confidence: MimeConfidence::ExactCustom,
            });
        }

        let best = self
            .combined
            .iter()
            .filter_map(|candidate| {
                let similarity = calculate_mime_similarity(&cleaned, candidate);
                (similarity > FUZZY_THRESHOLD).then_some((candidate, similarity))
            })
            .max_by(|left, right| {
                left.1
                    .total_cmp(&right.1)
                    .then_with(|| left.0.cmp(right.0).reverse())
            })?;

        let confidence = if self.iana.contains(best.0) {
            MimeConfidence::FuzzyIana
        } else {
            MimeConfidence::FuzzyCustom
        };

        Some(MimeResolution {
            canonical: best.0.clone(),
            confidence,
        })
    }
}

pub(crate) fn mime_catalog() -> &'static MimeCatalog {
    &MIME_CATALOG
}

pub(crate) fn clean_basic_mime_type(mime_type: &str) -> String {
    let cleaned = mime_type.trim().trim_matches([';', ',', '.', '"', '\'']);
    if cleaned.is_empty() {
        return String::new();
    }

    let lowered = cleaned.to_ascii_lowercase();
    let mut corrected = lowered
        .replace("aapplication/", "application/")
        .replace("applicaiton/", "application/")
        .replace("appliction/", "application/")
        .replace('\\', "/");
    corrected = corrected.replace(" /", "/").replace("/ ", "/");
    corrected
}

fn load_mime_list(contents: &str) -> HashSet<String> {
    contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.to_ascii_lowercase())
        .collect()
}

fn calculate_mime_similarity(value: &str, candidate: &str) -> f32 {
    let Some((value_type, value_subtype)) = value.split_once('/') else {
        return 0.0;
    };
    let Some((candidate_type, candidate_subtype)) = candidate.split_once('/') else {
        return 0.0;
    };

    let type_similarity = calculate_string_similarity(value_type, candidate_type);
    let subtype_similarity = calculate_string_similarity(value_subtype, candidate_subtype);
    type_similarity * 0.3 + subtype_similarity * 0.7
}

fn calculate_string_similarity(left: &str, right: &str) -> f32 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    if left.eq_ignore_ascii_case(right) {
        return 1.0;
    }

    let max_length = left.len().max(right.len());
    let distance = levenshtein_distance(left, right);
    1.0 - (distance as f32 / max_length as f32)
}

fn levenshtein_distance(left: &str, right: &str) -> usize {
    if left.is_empty() {
        return right.len();
    }
    if right.is_empty() {
        return left.len();
    }

    let left_chars = left.as_bytes();
    let right_chars = right.as_bytes();
    let mut previous = (0..=right_chars.len()).collect::<Vec<_>>();
    let mut current = vec![0; right_chars.len() + 1];

    for (row_index, left_char) in left_chars.iter().enumerate() {
        current[0] = row_index + 1;
        for (column_index, right_char) in right_chars.iter().enumerate() {
            let substitution_cost = usize::from(left_char != right_char);
            current[column_index + 1] = (previous[column_index + 1] + 1)
                .min(current[column_index] + 1)
                .min(previous[column_index] + substitution_cost);
        }
        previous.clone_from(&current);
    }

    previous[right_chars.len()]
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{MimeConfidence, clean_basic_mime_type, mime_catalog};

    #[test]
    fn cleans_common_mime_typos() {
        assert_eq!(
            clean_basic_mime_type("  Applicaiton/PDF; "),
            "application/pdf"
        );
    }

    #[test]
    fn canonicalizes_exact_and_fuzzy_mime_types() {
        let catalog = mime_catalog();
        let mut cache = HashMap::new();

        let exact = catalog
            .canonicalize("application/pdf", &mut cache)
            .expect("exact MIME type should resolve");
        assert_eq!(exact.canonical, "application/pdf");
        assert_eq!(exact.confidence, MimeConfidence::ExactIana);

        let corrected = catalog
            .canonicalize("applicaiton/pdf", &mut cache)
            .expect("typo should be repaired");
        assert_eq!(corrected.canonical, "application/pdf");
        assert_eq!(corrected.confidence, MimeConfidence::ExactIana);

        let fuzzy = catalog
            .canonicalize("application/pdff", &mut cache)
            .expect("fuzzy MIME type should resolve");
        assert_eq!(fuzzy.canonical, "application/pdf");
        assert_eq!(fuzzy.confidence, MimeConfidence::FuzzyIana);
    }

    #[test]
    fn rejects_unrecognized_mime_types() {
        let catalog = mime_catalog();
        let mut cache = HashMap::new();

        assert!(
            catalog
                .canonicalize("definitely/not-a-real-type", &mut cache)
                .is_none()
        );
    }
}
