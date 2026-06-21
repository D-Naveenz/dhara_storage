use std::path::Path;

use quick_xml::de::from_str;
use serde::Deserialize;

use crate::builder::BuilderError;

use super::{ParsedTridDefinition, TridPattern, TridSignature};

#[derive(Debug, Deserialize)]
struct TridXmlDocument {
    #[serde(rename = "@ver", default)]
    version: String,
    #[serde(rename = "Info")]
    info: TridXmlInfo,
    #[serde(rename = "General", default)]
    general: Option<TridXmlGeneral>,
    #[serde(rename = "FrontBlock", default)]
    front_block: Option<TridXmlPatternBlock>,
    #[serde(rename = "GlobalStrings", default)]
    global_strings: Option<TridXmlStrings>,
}

#[derive(Debug, Default, Deserialize)]
struct TridXmlInfo {
    #[serde(rename = "FileType", default)]
    file_type: String,
    #[serde(rename = "Ext", default)]
    extensions: String,
    #[serde(rename = "Mime", default)]
    mime_type: String,
    #[serde(rename = "ExtraInfo", default)]
    extra_info: Option<TridXmlExtraInfo>,
}

#[derive(Debug, Default, Deserialize)]
struct TridXmlExtraInfo {
    #[serde(rename = "Rem", default)]
    remarks: Vec<String>,
    #[serde(rename = "RefURL", default)]
    reference_url: String,
}

#[derive(Debug, Default, Deserialize)]
struct TridXmlGeneral {
    #[serde(rename = "FileNum", default)]
    file_count: u32,
    #[serde(rename = "Refine", default)]
    refine: String,
}

#[derive(Debug, Default, Deserialize)]
struct TridXmlPatternBlock {
    #[serde(rename = "Pattern", default)]
    patterns: Vec<TridXmlPattern>,
}

#[derive(Debug, Default, Deserialize)]
struct TridXmlPattern {
    #[serde(rename = "Bytes", default)]
    bytes: String,
    #[serde(rename = "Pos", default)]
    position: u16,
}

#[derive(Debug, Default, Deserialize)]
struct TridXmlStrings {
    #[serde(rename = "String", default)]
    strings: Vec<String>,
}

pub(crate) fn parse_trid_xml_definition(
    xml: &str,
    source_path: &Path,
) -> Result<ParsedTridDefinition, BuilderError> {
    let source_path = source_path.to_path_buf();
    let document: TridXmlDocument = from_str(xml).map_err(|error| BuilderError::Xml {
        path: source_path.clone(),
        message: error.to_string(),
    })?;

    let remarks = build_remarks(
        document
            .info
            .extra_info
            .as_ref()
            .map(|extra| extra.remarks.as_slice())
            .unwrap_or_default(),
        document
            .general
            .as_ref()
            .map(|general| general.refine.as_str())
            .unwrap_or_default(),
        document
            .info
            .extra_info
            .as_ref()
            .map(|extra| extra.reference_url.as_str())
            .unwrap_or_default(),
    );

    let patterns = document
        .front_block
        .unwrap_or_default()
        .patterns
        .into_iter()
        .filter(|pattern| !pattern.bytes.trim().is_empty())
        .map(|pattern| {
            decode_hex_bytes(&source_path, &pattern.bytes).map(|data| TridPattern {
                position: pattern.position,
                data,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let strings = document
        .global_strings
        .unwrap_or_default()
        .strings
        .into_iter()
        .map(|value: String| value.trim().to_string())
        .filter(|value: &String| !value.is_empty())
        .map(|value: String| value.into_bytes())
        .collect::<Vec<_>>();

    Ok(ParsedTridDefinition {
        source_version: document.version.trim().to_string(),
        file_type: document.info.file_type.trim().to_string(),
        extensions: split_extensions(&document.info.extensions),
        mime_type: normalize_mime_type(&document.info.mime_type),
        remarks,
        signature: TridSignature { patterns, strings },
        file_count: document.general.unwrap_or_default().file_count,
    })
}

fn split_extensions(value: &str) -> Vec<String> {
    let mut extensions = Vec::new();
    for part in value.split('/') {
        let trimmed = part.trim().trim_start_matches('.').to_ascii_lowercase();
        if trimmed.is_empty() || extensions.iter().any(|ext| ext == &trimmed) {
            continue;
        }
        extensions.push(trimmed);
    }
    extensions
}

fn normalize_mime_type(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "application/octet-stream".to_string()
    } else {
        trimmed.to_ascii_lowercase()
    }
}

fn build_remarks(remarks: &[String], refine: &str, reference_url: &str) -> String {
    let mut parts = Vec::new();
    for remark in remarks {
        let trimmed = remark.trim();
        if trimmed.is_empty() || parts.iter().any(|part| part == trimmed) {
            continue;
        }
        parts.push(trimmed.to_string());
    }
    if !refine.trim().is_empty() {
        parts.push(format!("Refine: {}", refine.trim()));
    }
    if !reference_url.trim().is_empty() {
        parts.push(format!("Reference: {}", reference_url.trim()));
    }
    parts.join("\n")
}

fn decode_hex_bytes(path: &Path, value: &str) -> Result<Vec<u8>, BuilderError> {
    let normalized = value
        .chars()
        .filter(|ch| !ch.is_ascii_whitespace())
        .collect::<String>();
    if normalized.len() % 2 != 0 {
        return Err(BuilderError::InvalidHex {
            path: path.to_path_buf(),
            value: value.to_string(),
        });
    }

    let mut bytes = Vec::with_capacity(normalized.len() / 2);
    let chars = normalized.as_bytes();
    for pair in chars.chunks_exact(2) {
        let chunk = std::str::from_utf8(pair).map_err(|_| BuilderError::InvalidHex {
            path: path.to_path_buf(),
            value: value.to_string(),
        })?;
        let byte = u8::from_str_radix(chunk, 16).map_err(|_| BuilderError::InvalidHex {
            path: path.to_path_buf(),
            value: value.to_string(),
        })?;
        bytes.push(byte);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::parse_trid_xml_definition;

    #[test]
    fn parse_trid_xml_definition_reads_patterns_and_strings() {
        let xml = r#"
<TrID ver="2.00">
    <Info>
        <FileType>Portable Network Graphics</FileType>
        <Ext>PNG</Ext>
        <Mime>image/png</Mime>
        <ExtraInfo>
            <Rem>Portable image format</Rem>
            <RefURL>https://example.com/png</RefURL>
        </ExtraInfo>
    </Info>
    <General>
        <FileNum>569</FileNum>
        <Refine>3 by Marco Pontello</Refine>
    </General>
    <FrontBlock>
        <Pattern>
            <Bytes>89504E470D0A1A0A</Bytes>
            <Pos>0</Pos>
        </Pattern>
    </FrontBlock>
    <GlobalStrings>
        <String>IHDR</String>
    </GlobalStrings>
</TrID>
"#;

        let parsed =
            parse_trid_xml_definition(xml, "fixture.trid.xml".as_ref()).expect("xml should parse");

        assert_eq!(parsed.source_version, "2.00");
        assert_eq!(parsed.file_type, "Portable Network Graphics");
        assert_eq!(parsed.extensions, vec!["png"]);
        assert_eq!(parsed.mime_type, "image/png");
        assert!(parsed.remarks.contains("Portable image format"));
        assert!(parsed.remarks.contains("Refine: 3 by Marco Pontello"));
        assert_eq!(parsed.file_count, 569);
        assert_eq!(parsed.signature.patterns.len(), 1);
        assert_eq!(parsed.signature.patterns[0].position, 0);
        assert_eq!(
            parsed.signature.patterns[0].data,
            vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
        );
        assert_eq!(parsed.signature.strings, vec![b"IHDR".to_vec()]);
    }

    #[test]
    fn split_extensions_normalizes_slash_separated_values() {
        let xml = r#"
<TrID ver="2.00">
    <Info>
        <FileType>Disc Image</FileType>
        <Ext>ISO/UDF/.IMG</Ext>
        <Mime>application/x-udf-image</Mime>
    </Info>
    <General>
        <FileNum>12</FileNum>
    </General>
    <FrontBlock>
        <Pattern>
            <Bytes>41424344</Bytes>
            <Pos>0</Pos>
        </Pattern>
    </FrontBlock>
</TrID>
"#;

        let parsed =
            parse_trid_xml_definition(xml, "fixture.trid.xml".as_ref()).expect("xml should parse");

        assert_eq!(parsed.extensions, vec!["iso", "udf", "img"]);
    }
}
