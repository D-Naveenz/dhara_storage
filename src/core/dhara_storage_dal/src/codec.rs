use crate::error::DefinitionPackageError;
use crate::generated::dhara::storage::dal as fb;
use crate::model::{
    DefinitionPackage, DefinitionPackageView, DefinitionRecord, SignatureDefinition,
    SignaturePattern,
};

/// Encode the FlatBuffers payload section for a definition package.
pub(crate) fn encode_flatbuffer_payload(package: &DefinitionPackage) -> Vec<u8> {
    let mut builder = flatbuffers::FlatBufferBuilder::with_capacity(estimate_capacity(package));

    let definitions = package
        .definitions
        .iter()
        .map(|definition| encode_definition_record(&mut builder, definition))
        .collect::<Vec<_>>();
    let definitions = builder.create_vector(&definitions);
    let root = fb::DefinitionPackage::create(
        &mut builder,
        &fb::DefinitionPackageArgs {
            package_revision: package.package_revision,
            tags: package.tags,
            definitions: Some(definitions),
        },
    );
    fb::finish_definition_package_buffer(&mut builder, root);
    builder.finished_data().to_vec()
}

/// Decode a FlatBuffers payload section into an owned definition package.
///
/// Metadata fields (`package_version`, `definitions_release`) are left empty and
/// must be filled by the container decoder from the XML footer.
pub(crate) fn decode_flatbuffer_payload(
    bytes: &[u8],
) -> Result<DefinitionPackage, DefinitionPackageError> {
    Ok(owned_package(root_flatbuffer_package(bytes)?))
}

/// Return a verified borrowed FlatBuffers root view over a payload buffer.
pub(crate) fn root_flatbuffer_package(
    bytes: &[u8],
) -> Result<DefinitionPackageView<'_>, DefinitionPackageError> {
    Ok(fb::root_as_definition_package(bytes)?)
}

fn encode_definition_record<'a>(
    builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    definition: &DefinitionRecord,
) -> flatbuffers::WIPOffset<fb::DefinitionRecord<'a>> {
    let file_type = builder.create_string(&definition.file_type);
    let extensions = definition
        .extensions
        .iter()
        .map(|extension| builder.create_string(extension))
        .collect::<Vec<_>>();
    let extensions = builder.create_vector(&extensions);
    let mime_type = builder.create_string(&definition.mime_type);
    let remarks = builder.create_string(&definition.remarks);
    let signature = encode_signature_definition(builder, &definition.signature);

    fb::DefinitionRecord::create(
        builder,
        &fb::DefinitionRecordArgs {
            file_type: Some(file_type),
            extensions: Some(extensions),
            mime_type: Some(mime_type),
            remarks: Some(remarks),
            signature: Some(signature),
            priority_level: definition.priority_level,
        },
    )
}

fn encode_signature_definition<'a>(
    builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    signature: &SignatureDefinition,
) -> flatbuffers::WIPOffset<fb::SignatureDefinition<'a>> {
    let patterns = signature
        .patterns
        .iter()
        .map(|pattern| {
            let data = builder.create_vector(pattern.data.as_slice());
            fb::SignaturePattern::create(
                builder,
                &fb::SignaturePatternArgs {
                    position: pattern.position,
                    data: Some(data),
                },
            )
        })
        .collect::<Vec<_>>();
    let patterns = builder.create_vector(&patterns);

    let strings = signature
        .strings
        .iter()
        .map(|value| {
            let data = builder.create_vector(value.as_slice());
            fb::ByteBlob::create(builder, &fb::ByteBlobArgs { data: Some(data) })
        })
        .collect::<Vec<_>>();
    let strings = builder.create_vector(&strings);

    fb::SignatureDefinition::create(
        builder,
        &fb::SignatureDefinitionArgs {
            patterns: Some(patterns),
            strings: Some(strings),
        },
    )
}

fn owned_package(package: fb::DefinitionPackage<'_>) -> DefinitionPackage {
    DefinitionPackage {
        package_version: String::new(),
        definitions_release: String::new(),
        package_revision: package.package_revision(),
        tags: package.tags(),
        definitions: package
            .definitions()
            .map(|definitions| definitions.iter().map(owned_definition).collect())
            .unwrap_or_default(),
    }
}

fn owned_definition(definition: fb::DefinitionRecord<'_>) -> DefinitionRecord {
    DefinitionRecord {
        file_type: definition.file_type().unwrap_or_default().to_owned(),
        extensions: definition
            .extensions()
            .map(|extensions| extensions.iter().map(ToOwned::to_owned).collect())
            .unwrap_or_default(),
        mime_type: definition.mime_type().unwrap_or_default().to_owned(),
        remarks: definition.remarks().unwrap_or_default().to_owned(),
        signature: definition
            .signature()
            .map(owned_signature)
            .unwrap_or_default(),
        priority_level: definition.priority_level(),
    }
}

fn owned_signature(signature: fb::SignatureDefinition<'_>) -> SignatureDefinition {
    SignatureDefinition {
        patterns: signature
            .patterns()
            .map(|patterns| patterns.iter().map(owned_pattern).collect())
            .unwrap_or_default(),
        strings: signature
            .strings()
            .map(|strings| {
                strings
                    .iter()
                    .map(|value| {
                        value
                            .data()
                            .map(|data| data.iter().collect::<Vec<_>>())
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn owned_pattern(pattern: fb::SignaturePattern<'_>) -> SignaturePattern {
    SignaturePattern {
        position: pattern.position(),
        data: pattern
            .data()
            .map(|data| data.iter().collect::<Vec<_>>())
            .unwrap_or_default(),
    }
}

fn estimate_capacity(package: &DefinitionPackage) -> usize {
    let string_bytes = package
        .definitions
        .iter()
        .map(|definition| {
            definition.file_type.len()
                + definition.mime_type.len()
                + definition.remarks.len()
                + definition.extensions.iter().map(String::len).sum::<usize>()
        })
        .sum::<usize>();
    let byte_bytes = package
        .definitions
        .iter()
        .map(|definition| {
            definition
                .signature
                .patterns
                .iter()
                .map(|pattern| pattern.data.len())
                .sum::<usize>()
                + definition
                    .signature
                    .strings
                    .iter()
                    .map(Vec::len)
                    .sum::<usize>()
        })
        .sum::<usize>();
    (string_bytes + byte_bytes + package.definitions.len() * 128).max(1024)
}

#[cfg(test)]
mod tests {
    use super::{decode_flatbuffer_payload, encode_flatbuffer_payload, root_flatbuffer_package};
    use crate::error::DefinitionPackageError;
    use crate::model::{
        DefinitionPackage, DefinitionRecord, SignatureDefinition, SignaturePattern,
    };

    fn sample_package() -> DefinitionPackage {
        DefinitionPackage {
            package_version: "0.6.0".to_owned(),
            definitions_release: "2026-06-24".to_owned(),
            package_revision: 1,
            tags: 48,
            definitions: vec![DefinitionRecord {
                file_type: "Portable Network Graphics".to_owned(),
                extensions: vec!["png".to_owned()],
                mime_type: "image/png".to_owned(),
                remarks: "fixture".to_owned(),
                signature: SignatureDefinition {
                    patterns: vec![SignaturePattern {
                        position: 0,
                        data: vec![0x89, b'P', b'N', b'G'],
                    }],
                    strings: vec![b"IHDR".to_vec()],
                },
                priority_level: 42,
            }],
        }
    }

    #[test]
    fn flatbuffer_roundtrip_preserves_payload_semantics() {
        let package = sample_package();
        let encoded = encode_flatbuffer_payload(&package);
        let decoded = decode_flatbuffer_payload(&encoded).expect("payload should decode");

        assert_eq!(decoded.package_revision, package.package_revision);
        assert_eq!(decoded.tags, package.tags);
        assert_eq!(decoded.definitions, package.definitions);
        assert!(decoded.package_version.is_empty());
        assert!(decoded.definitions_release.is_empty());
    }

    #[test]
    fn root_view_reads_without_owned_decode() {
        let encoded = encode_flatbuffer_payload(&sample_package());
        let root = root_flatbuffer_package(&encoded).expect("root should verify");

        assert_eq!(root.definitions().unwrap().len(), 1);
        assert_eq!(root.package_revision(), 1);
    }

    #[test]
    fn malformed_payload_is_rejected() {
        let error = decode_flatbuffer_payload(b"not-flatbuffers").unwrap_err();

        assert!(matches!(
            error,
            DefinitionPackageError::InvalidFlatbuffer(_)
        ));
    }
}
