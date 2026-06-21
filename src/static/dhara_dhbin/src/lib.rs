#![deny(missing_docs)]

//! Generic MessagePack-based `DHBIN` v2 container support.
//!
//! This crate provides the shared package reader and writer used by the Dhara
//! workspace. It keeps container framing, compression, metadata, and integrity
//! concerns out of higher-level runtime crates so they can focus on their own
//! domain models.

use std::io::{Cursor, Read, Write};

use lz4_flex::frame::{FrameDecoder, FrameEncoder};
use serde::Serialize;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// The four-byte magic header that identifies a `DHBIN` container.
pub const DHBIN_MAGIC: &[u8; 4] = b"DHBN";
/// The currently supported on-disk wire-format version.
pub const DHBIN_WIRE_VERSION: u8 = 2;
const HEADER_LEN: usize = 28;
const SECTION_DESCRIPTOR_LEN: usize = 24;
const PAYLOAD_FORMAT_MESSAGEPACK: u8 = 1;
const METADATA_FORMAT_MESSAGEPACK: u8 = 1;
const INTEGRITY_FORMAT_SHA256: u8 = 1;

/// Describes how readers should treat the package by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackagePurpose {
    /// Fully featured package intended for standard read paths.
    Standard = 0,
    /// Package optimized for fast payload reads where metadata is usually optional.
    FastPayload = 1,
    /// Package optimized for embedded runtime assets with minimal startup overhead.
    Embedded = 2,
}

impl PackagePurpose {
    /// Returns the default read options recommended for this package purpose.
    ///
    /// # Returns
    ///
    /// - [`DhbinReadOptions`] - The default metadata and integrity policy for the purpose.
    pub fn default_read_options(self) -> DhbinReadOptions {
        match self {
            Self::Standard => DhbinReadOptions::default(),
            Self::FastPayload | Self::Embedded => DhbinReadOptions {
                verify_integrity: false,
                load_metadata: false,
            },
        }
    }
}

impl TryFrom<u8> for PackagePurpose {
    type Error = DhbinDecodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Standard),
            1 => Ok(Self::FastPayload),
            2 => Ok(Self::Embedded),
            _ => Err(DhbinDecodeError::UnsupportedPurpose(value)),
        }
    }
}

/// Compression algorithm applied to the payload section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionKind {
    /// The payload is stored without compression.
    None = 0,
    /// The payload is compressed using an LZ4 frame.
    Lz4Frame = 1,
}

impl TryFrom<u8> for CompressionKind {
    type Error = DhbinDecodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Lz4Frame),
            _ => Err(DhbinDecodeError::UnsupportedCompression(value)),
        }
    }
}

/// Integrity algorithm used for the package integrity section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrityKind {
    /// No integrity section is stored.
    None = 0,
    /// The integrity section stores a SHA-256 digest.
    Sha256 = 1,
}

impl TryFrom<u8> for IntegrityKind {
    type Error = DhbinDecodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Sha256),
            _ => Err(DhbinDecodeError::UnsupportedIntegrity(value)),
        }
    }
}

/// Logical section type stored in the package section table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionKind {
    /// The primary MessagePack payload section.
    Payload = 1,
    /// Optional MessagePack metadata section.
    Metadata = 2,
    /// Optional integrity section.
    Integrity = 3,
    /// Reserved section kind for future chunk index support.
    ChunkIndex = 4,
}

impl TryFrom<u8> for SectionKind {
    type Error = DhbinDecodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Payload),
            2 => Ok(Self::Metadata),
            3 => Ok(Self::Integrity),
            4 => Ok(Self::ChunkIndex),
            _ => Err(DhbinDecodeError::UnsupportedSectionKind(value)),
        }
    }
}

/// Parsed header values from the fixed-size `DHBIN` package header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageHeader {
    /// Wire-format version stored in the header.
    pub wire_version: u8,
    /// Four-byte application-level package identifier.
    pub package_id: [u8; 4],
    /// Package purpose advertised by the writer.
    pub purpose: PackagePurpose,
    /// Compression applied to the payload section.
    pub compression: CompressionKind,
    /// Writer-controlled flags reserved for higher-level packages.
    pub flags: u8,
    /// Number of section descriptors in the section table.
    pub section_count: u16,
    /// Byte offset of the section table from the start of the container.
    pub section_table_offset: u32,
    /// Total length in bytes of the section table.
    pub section_table_len: u32,
}

/// Single section-table entry describing one payload, metadata, or integrity section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionDescriptor {
    /// Logical section type.
    pub kind: SectionKind,
    /// Format discriminator for the section contents.
    pub format: u8,
    /// Byte offset of the section contents from the start of the package.
    pub offset: u64,
    /// Length in bytes of the section contents.
    pub length: u64,
}

/// Controls optional integrity and metadata loading during package reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DhbinReadOptions {
    /// Whether the reader should verify the integrity section when present.
    pub verify_integrity: bool,
    /// Whether the reader should materialize metadata bytes when present.
    pub load_metadata: bool,
}

impl Default for DhbinReadOptions {
    fn default() -> Self {
        Self {
            verify_integrity: true,
            load_metadata: true,
        }
    }
}

/// Configures how a new `DHBIN` package should be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DhbinWriteOptions {
    /// Four-byte application-level package identifier.
    pub package_id: [u8; 4],
    /// Intended package purpose for downstream readers.
    pub purpose: PackagePurpose,
    /// Compression algorithm to use for the payload section.
    pub compression: CompressionKind,
    /// Writer-controlled flags reserved for higher-level consumers.
    pub flags: u8,
    /// Optional MessagePack metadata bytes to emit as a metadata section.
    pub metadata: Option<Vec<u8>>,
    /// Integrity section policy for the written package.
    pub integrity: IntegrityKind,
}

/// Fully decoded `DHBIN` container with payload bytes and optional metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedPackage {
    /// Parsed header values.
    pub header: PackageHeader,
    /// Parsed section descriptors from the section table.
    pub sections: Vec<SectionDescriptor>,
    /// Decompressed payload bytes.
    pub payload: Vec<u8>,
    /// Optional metadata bytes when metadata loading was enabled.
    pub metadata: Option<Vec<u8>>,
    /// Integrity kind reported by the container.
    pub integrity: IntegrityKind,
    /// Whether the integrity section was verified successfully during the read.
    pub integrity_verified: bool,
}

/// Errors that can occur while decoding a `DHBIN` container.
#[derive(Debug, Error)]
pub enum DhbinDecodeError {
    /// The input did not begin with the `DHBIN` magic header.
    #[error("payload is not a DHBIN v2 container")]
    UnsupportedFormat,
    /// The input ended before the fixed-size header could be read fully.
    #[error("payload is too short for a DHBIN header")]
    TruncatedHeader,
    /// The container advertised a wire-format version that this crate does not support.
    #[error("unsupported DHBIN wire version: {0}")]
    UnsupportedWireVersion(u8),
    /// The package purpose byte did not map to a known [`PackagePurpose`] value.
    #[error("unsupported DHBIN purpose: {0}")]
    UnsupportedPurpose(u8),
    /// The payload compression byte did not map to a known [`CompressionKind`] value.
    #[error("unsupported DHBIN compression kind: {0}")]
    UnsupportedCompression(u8),
    /// The integrity-section format byte did not map to a known [`IntegrityKind`] value.
    #[error("unsupported DHBIN integrity kind: {0}")]
    UnsupportedIntegrity(u8),
    /// A section-table entry used an unknown [`SectionKind`] discriminator.
    #[error("unsupported DHBIN section kind: {0}")]
    UnsupportedSectionKind(u8),
    /// The section table offset, length, or entry count was internally inconsistent.
    #[error("DHBIN section table is malformed")]
    InvalidSectionTable,
    /// No payload section was present in the section table.
    #[error("DHBIN payload section is missing")]
    MissingPayloadSection,
    /// The metadata section length or format was invalid.
    #[error("DHBIN metadata section is malformed")]
    InvalidMetadataSection,
    /// The integrity section length or format was invalid.
    #[error("DHBIN integrity section is malformed")]
    InvalidIntegritySection,
    /// A section offset or length pointed outside the container byte range.
    #[error("DHBIN section bytes are out of range")]
    InvalidSectionRange,
    /// The payload section did not use the expected MessagePack format discriminator.
    #[error("DHBIN payload section does not use MessagePack format")]
    InvalidPayloadFormat,
    /// The metadata section did not use the expected MessagePack format discriminator.
    #[error("DHBIN metadata section does not use MessagePack format")]
    InvalidMetadataFormat,
    /// The stored integrity digest did not match the recomputed payload digest.
    #[error("DHBIN payload checksum does not match integrity section")]
    ChecksumMismatch,
    /// Decompression of the payload section failed.
    #[error("failed to decompress DHBIN payload: {0}")]
    Compression(#[from] std::io::Error),
    /// MessagePack deserialization of the payload failed.
    #[error("failed to decode MessagePack payload: {0}")]
    MessagePack(#[from] rmp_serde::decode::Error),
}

/// Errors that can occur while encoding a `DHBIN` container.
#[derive(Debug, Error)]
pub enum DhbinEncodeError {
    /// MessagePack serialization of the payload or metadata failed.
    #[error("failed to encode MessagePack payload: {0}")]
    MessagePack(#[from] rmp_serde::encode::Error),
    /// Compression of the payload section failed.
    #[error("failed to compress DHBIN payload: {0}")]
    Compression(#[from] std::io::Error),
    /// Finalization of the LZ4 frame failed after compression.
    #[error("failed to finalize DHBIN compression frame: {0}")]
    CompressionFrame(#[from] lz4_flex::frame::Error),
}

/// Reader utilities for parsing `DHBIN` package headers, sections, and payloads.
pub struct DhbinReader;

impl DhbinReader {
    /// Parse the fixed-size package header from raw container bytes.
    ///
    /// # Arguments
    ///
    /// - `bytes` (`&[u8]`) - The raw bytes to inspect.
    ///
    /// # Returns
    ///
    /// - `Result<PackageHeader, DhbinDecodeError>` - The parsed header when the container prefix is valid.
    ///
    /// # Errors
    ///
    /// Returns an error when the payload is not an `DHBIN` container, is truncated,
    /// or advertises unsupported header values.
    pub fn read_header(bytes: &[u8]) -> Result<PackageHeader, DhbinDecodeError> {
        if bytes.len() < HEADER_LEN {
            if bytes.starts_with(DHBIN_MAGIC) {
                return Err(DhbinDecodeError::TruncatedHeader);
            }
            return Err(DhbinDecodeError::UnsupportedFormat);
        }
        if &bytes[..4] != DHBIN_MAGIC {
            return Err(DhbinDecodeError::UnsupportedFormat);
        }
        let wire_version = bytes[4];
        if wire_version != DHBIN_WIRE_VERSION {
            return Err(DhbinDecodeError::UnsupportedWireVersion(wire_version));
        }
        Ok(PackageHeader {
            wire_version,
            purpose: PackagePurpose::try_from(bytes[5])?,
            compression: CompressionKind::try_from(bytes[6])?,
            flags: bytes[7],
            package_id: [bytes[8], bytes[9], bytes[10], bytes[11]],
            section_count: u16::from_le_bytes(bytes[12..14].try_into().unwrap()),
            section_table_offset: u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
            section_table_len: u32::from_le_bytes(bytes[20..24].try_into().unwrap()),
        })
    }

    /// Parse the section table described by a previously decoded header.
    ///
    /// # Arguments
    ///
    /// - `bytes` (`&[u8]`) - The raw container bytes.
    /// - `header` (`&PackageHeader`) - The parsed package header that describes the section table.
    ///
    /// # Returns
    ///
    /// - `Result<Vec<SectionDescriptor>, DhbinDecodeError>` - The decoded section descriptors.
    ///
    /// # Errors
    ///
    /// Returns an error when the section table is malformed, truncated, or advertises unsupported section kinds.
    pub fn read_sections(
        bytes: &[u8],
        header: &PackageHeader,
    ) -> Result<Vec<SectionDescriptor>, DhbinDecodeError> {
        let section_table_offset = usize::try_from(header.section_table_offset)
            .map_err(|_| DhbinDecodeError::InvalidSectionTable)?;
        let section_table_len = usize::try_from(header.section_table_len)
            .map_err(|_| DhbinDecodeError::InvalidSectionTable)?;
        let table_end = section_table_offset
            .checked_add(section_table_len)
            .ok_or(DhbinDecodeError::InvalidSectionTable)?;
        if table_end > bytes.len() || section_table_offset < HEADER_LEN {
            return Err(DhbinDecodeError::InvalidSectionTable);
        }
        if section_table_len != header.section_count as usize * SECTION_DESCRIPTOR_LEN {
            return Err(DhbinDecodeError::InvalidSectionTable);
        }

        let mut sections = Vec::with_capacity(header.section_count as usize);
        let mut cursor = section_table_offset;
        for _ in 0..header.section_count {
            let entry = &bytes[cursor..cursor + SECTION_DESCRIPTOR_LEN];
            sections.push(SectionDescriptor {
                kind: SectionKind::try_from(entry[0])?,
                format: entry[1],
                offset: u64::from_le_bytes(entry[4..12].try_into().unwrap()),
                length: u64::from_le_bytes(entry[12..20].try_into().unwrap()),
            });
            cursor += SECTION_DESCRIPTOR_LEN;
        }
        Ok(sections)
    }

    /// Read and decompress the payload section from a raw container.
    ///
    /// # Arguments
    ///
    /// - `bytes` (`&[u8]`) - The raw container bytes.
    ///
    /// # Returns
    ///
    /// - `Result<Vec<u8>, DhbinDecodeError>` - The decoded payload bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the container cannot be parsed, when the payload section
    /// is missing or malformed, or when decompression fails.
    pub fn read_payload_bytes(bytes: &[u8]) -> Result<Vec<u8>, DhbinDecodeError> {
        let header = Self::read_header(bytes)?;
        let sections = Self::read_sections(bytes, &header)?;
        let payload = find_section(&sections, SectionKind::Payload)
            .ok_or(DhbinDecodeError::MissingPayloadSection)?;
        if payload.format != PAYLOAD_FORMAT_MESSAGEPACK {
            return Err(DhbinDecodeError::InvalidPayloadFormat);
        }
        let payload_slice = read_section_bytes(bytes, payload)?;
        decompress_payload(payload_slice, header.compression)
    }

    /// Read a full package with caller-controlled metadata and integrity behavior.
    ///
    /// # Arguments
    ///
    /// - `bytes` (`&[u8]`) - The raw container bytes.
    /// - `options` (`&DhbinReadOptions`) - The read behavior to apply.
    ///
    /// # Returns
    ///
    /// - `Result<DecodedPackage, DhbinDecodeError>` - The decoded package contents.
    ///
    /// # Errors
    ///
    /// Returns an error when the container is malformed, when integrity verification
    /// fails, or when the payload cannot be decompressed.
    pub fn read_package(
        bytes: &[u8],
        options: &DhbinReadOptions,
    ) -> Result<DecodedPackage, DhbinDecodeError> {
        let header = Self::read_header(bytes)?;
        let sections = Self::read_sections(bytes, &header)?;
        let payload_desc = find_section(&sections, SectionKind::Payload)
            .ok_or(DhbinDecodeError::MissingPayloadSection)?;
        if payload_desc.format != PAYLOAD_FORMAT_MESSAGEPACK {
            return Err(DhbinDecodeError::InvalidPayloadFormat);
        }

        let effective_load_metadata = options.load_metadata;
        let effective_verify_integrity = options.verify_integrity;

        let payload_slice = read_section_bytes(bytes, payload_desc)?;
        let integrity_desc = find_section(&sections, SectionKind::Integrity);
        let integrity = match integrity_desc {
            Some(section) => IntegrityKind::try_from(section.format)?,
            None => IntegrityKind::None,
        };

        if effective_verify_integrity && integrity == IntegrityKind::Sha256 {
            verify_integrity(bytes, &header, &sections, payload_desc, integrity_desc)?;
        }

        let metadata = if effective_load_metadata {
            if let Some(section) = find_section(&sections, SectionKind::Metadata) {
                if section.format != METADATA_FORMAT_MESSAGEPACK {
                    return Err(DhbinDecodeError::InvalidMetadataFormat);
                }
                Some(read_section_bytes(bytes, section)?.to_vec())
            } else {
                None
            }
        } else {
            None
        };

        let compression = header.compression;
        Ok(DecodedPackage {
            header,
            sections,
            payload: decompress_payload(payload_slice, compression)?,
            metadata,
            integrity,
            integrity_verified: effective_verify_integrity && integrity == IntegrityKind::Sha256,
        })
    }

    /// Read a package using the default policy advertised by its [`PackagePurpose`].
    ///
    /// # Arguments
    ///
    /// - `bytes` (`&[u8]`) - The raw container bytes.
    ///
    /// # Returns
    ///
    /// - `Result<DecodedPackage, DhbinDecodeError>` - The decoded package contents.
    ///
    /// # Errors
    ///
    /// Returns an error when the header cannot be read or the package fails to decode
    /// under its purpose-specific default options.
    pub fn read_package_default(bytes: &[u8]) -> Result<DecodedPackage, DhbinDecodeError> {
        let header = Self::read_header(bytes)?;
        let options = header.purpose.default_read_options();
        Self::read_package(bytes, &options)
    }

    /// Decode the payload section into a strongly typed MessagePack value.
    ///
    /// # Arguments
    ///
    /// - `bytes` (`&[u8]`) - The raw container bytes.
    ///
    /// # Returns
    ///
    /// - `Result<T, DhbinDecodeError>` - The deserialized payload value.
    ///
    /// # Errors
    ///
    /// Returns an error when the payload cannot be extracted, decompressed, or deserialized.
    pub fn decode_payload<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, DhbinDecodeError> {
        let payload = Self::read_payload_bytes(bytes)?;
        Ok(rmp_serde::from_slice(&payload)?)
    }

    /// Decode the payload section into a strongly typed value using explicit read options.
    ///
    /// # Arguments
    ///
    /// - `bytes` (`&[u8]`) - The raw container bytes.
    /// - `options` (`&DhbinReadOptions`) - The read behavior to apply before deserialization.
    ///
    /// # Returns
    ///
    /// - `Result<T, DhbinDecodeError>` - The deserialized payload value.
    ///
    /// # Errors
    ///
    /// Returns an error when package decoding or MessagePack deserialization fails.
    pub fn decode_payload_with_options<T: DeserializeOwned>(
        bytes: &[u8],
        options: &DhbinReadOptions,
    ) -> Result<T, DhbinDecodeError> {
        let package = Self::read_package(bytes, options)?;
        Ok(rmp_serde::from_slice(&package.payload)?)
    }
}

/// Writer utilities for creating `DHBIN` packages from raw or typed MessagePack payloads.
pub struct DhbinWriter;

impl DhbinWriter {
    /// Write a `DHBIN` package from raw MessagePack payload bytes.
    ///
    /// # Arguments
    ///
    /// - `payload` (`&[u8]`) - The MessagePack payload bytes to store.
    /// - `options` (`&DhbinWriteOptions`) - The package layout and integrity options.
    ///
    /// # Returns
    ///
    /// - `Result<Vec<u8>, DhbinEncodeError>` - The encoded container bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when compression or integrity-frame finalization fails.
    pub fn write_payload_bytes(
        payload: &[u8],
        options: &DhbinWriteOptions,
    ) -> Result<Vec<u8>, DhbinEncodeError> {
        let payload_section = match options.compression {
            CompressionKind::None => payload.to_vec(),
            CompressionKind::Lz4Frame => {
                let mut encoder = FrameEncoder::new(Vec::new());
                encoder.write_all(payload)?;
                encoder.finish()?
            }
        };

        let mut sections = Vec::with_capacity(3);
        sections.push((
            SectionKind::Payload,
            PAYLOAD_FORMAT_MESSAGEPACK,
            payload_section,
        ));
        if let Some(metadata) = &options.metadata {
            sections.push((
                SectionKind::Metadata,
                METADATA_FORMAT_MESSAGEPACK,
                metadata.clone(),
            ));
        }
        if options.integrity == IntegrityKind::Sha256 {
            sections.push((SectionKind::Integrity, INTEGRITY_FORMAT_SHA256, vec![0; 32]));
        }

        let section_count = sections.len() as u16;
        let section_table_offset = HEADER_LEN as u32;
        let section_table_len = section_count as u32 * SECTION_DESCRIPTOR_LEN as u32;

        let mut current_offset = HEADER_LEN + section_table_len as usize;
        let mut descriptors = Vec::with_capacity(sections.len());
        for (kind, format, data) in &sections {
            descriptors.push(SectionDescriptor {
                kind: *kind,
                format: *format,
                offset: current_offset as u64,
                length: data.len() as u64,
            });
            current_offset += data.len();
        }

        let header = PackageHeader {
            wire_version: DHBIN_WIRE_VERSION,
            package_id: options.package_id,
            purpose: options.purpose,
            compression: options.compression,
            flags: options.flags,
            section_count,
            section_table_offset,
            section_table_len,
        };

        let mut bytes = Vec::with_capacity(current_offset);
        bytes.extend_from_slice(&encode_header(&header));
        bytes.extend_from_slice(&encode_sections(&descriptors));
        for (_, _, data) in &sections {
            bytes.extend_from_slice(data);
        }

        if let Some(integrity_index) = descriptors
            .iter()
            .position(|descriptor| descriptor.kind == SectionKind::Integrity)
        {
            let digest = compute_integrity_digest(&bytes, &header, &descriptors, &descriptors[0]);
            let integrity = &descriptors[integrity_index];
            let start = integrity.offset as usize;
            bytes[start..start + 32].copy_from_slice(&digest);
        }

        Ok(bytes)
    }

    /// Serialize a value to MessagePack and write it into a `DHBIN` container.
    ///
    /// # Arguments
    ///
    /// - `value` (`&T`) - The value to serialize as the package payload.
    /// - `options` (`&DhbinWriteOptions`) - The package layout and integrity options.
    ///
    /// # Returns
    ///
    /// - `Result<Vec<u8>, DhbinEncodeError>` - The encoded container bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when MessagePack serialization, compression, or integrity-frame
    /// finalization fails.
    pub fn write_payload<T: Serialize>(
        value: &T,
        options: &DhbinWriteOptions,
    ) -> Result<Vec<u8>, DhbinEncodeError> {
        let payload = rmp_serde::to_vec(value)?;
        Self::write_payload_bytes(&payload, options)
    }
}

fn encode_header(header: &PackageHeader) -> [u8; HEADER_LEN] {
    let mut bytes = [0u8; HEADER_LEN];
    bytes[..4].copy_from_slice(DHBIN_MAGIC);
    bytes[4] = header.wire_version;
    bytes[5] = header.purpose as u8;
    bytes[6] = header.compression as u8;
    bytes[7] = header.flags;
    bytes[8..12].copy_from_slice(&header.package_id);
    bytes[12..14].copy_from_slice(&header.section_count.to_le_bytes());
    bytes[16..20].copy_from_slice(&header.section_table_offset.to_le_bytes());
    bytes[20..24].copy_from_slice(&header.section_table_len.to_le_bytes());
    bytes
}

fn encode_sections(descriptors: &[SectionDescriptor]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(descriptors.len() * SECTION_DESCRIPTOR_LEN);
    for descriptor in descriptors {
        bytes.push(descriptor.kind as u8);
        bytes.push(descriptor.format);
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&descriptor.offset.to_le_bytes());
        bytes.extend_from_slice(&descriptor.length.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
    }
    bytes
}

fn find_section(sections: &[SectionDescriptor], kind: SectionKind) -> Option<&SectionDescriptor> {
    sections.iter().find(|section| section.kind == kind)
}

fn read_section_bytes<'a>(
    bytes: &'a [u8],
    descriptor: &SectionDescriptor,
) -> Result<&'a [u8], DhbinDecodeError> {
    let start =
        usize::try_from(descriptor.offset).map_err(|_| DhbinDecodeError::InvalidSectionRange)?;
    let len =
        usize::try_from(descriptor.length).map_err(|_| DhbinDecodeError::InvalidSectionRange)?;
    let end = start
        .checked_add(len)
        .ok_or(DhbinDecodeError::InvalidSectionRange)?;
    if end > bytes.len() {
        return Err(DhbinDecodeError::InvalidSectionRange);
    }
    Ok(&bytes[start..end])
}

fn decompress_payload(
    payload: &[u8],
    compression: CompressionKind,
) -> Result<Vec<u8>, DhbinDecodeError> {
    match compression {
        CompressionKind::None => Ok(payload.to_vec()),
        CompressionKind::Lz4Frame => {
            let mut decoder = FrameDecoder::new(Cursor::new(payload));
            let mut output = Vec::new();
            decoder.read_to_end(&mut output)?;
            Ok(output)
        }
    }
}

fn verify_integrity(
    bytes: &[u8],
    header: &PackageHeader,
    sections: &[SectionDescriptor],
    payload: &SectionDescriptor,
    integrity: Option<&SectionDescriptor>,
) -> Result<(), DhbinDecodeError> {
    let Some(integrity) = integrity else {
        return Ok(());
    };
    if integrity.format != INTEGRITY_FORMAT_SHA256 || integrity.length != 32 {
        return Err(DhbinDecodeError::InvalidIntegritySection);
    }
    let expected = read_section_bytes(bytes, integrity)?;
    let actual = compute_integrity_digest(bytes, header, sections, payload);
    if actual.as_slice() != expected {
        return Err(DhbinDecodeError::ChecksumMismatch);
    }
    Ok(())
}

fn compute_integrity_digest(
    bytes: &[u8],
    header: &PackageHeader,
    sections: &[SectionDescriptor],
    payload: &SectionDescriptor,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(encode_header(header));
    hasher.update(encode_sections(sections));
    if let Ok(payload_bytes) = read_section_bytes(bytes, payload) {
        hasher.update(payload_bytes);
    }
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::{
        CompressionKind, DhbinDecodeError, DhbinReadOptions, DhbinReader, DhbinWriteOptions,
        DhbinWriter, IntegrityKind, METADATA_FORMAT_MESSAGEPACK, PackageHeader, PackagePurpose,
        SectionKind,
    };

    #[test]
    fn fast_profile_roundtrips_payload_without_metadata() {
        let bytes = DhbinWriter::write_payload(
            &vec!["alpha".to_string(), "beta".to_string()],
            &DhbinWriteOptions {
                package_id: *b"CONF",
                purpose: PackagePurpose::FastPayload,
                compression: CompressionKind::Lz4Frame,
                flags: 0,
                metadata: Some(rmp_serde::to_vec(&vec!["ignored"]).unwrap()),
                integrity: IntegrityKind::Sha256,
            },
        )
        .expect("package should encode");

        let payload: Vec<String> =
            DhbinReader::decode_payload(&bytes).expect("payload should decode");
        assert_eq!(payload, vec!["alpha", "beta"]);

        let package = DhbinReader::read_package_default(&bytes).expect("package should load");
        assert!(package.metadata.is_none());
        assert!(!package.integrity_verified);
    }

    #[test]
    fn full_profile_roundtrips_with_metadata_and_checksum() {
        let metadata = rmp_serde::to_vec(&vec![1u8, 2, 3]).unwrap();
        let bytes = DhbinWriter::write_payload(
            &vec!["alpha".to_string()],
            &DhbinWriteOptions {
                package_id: *b"CONF",
                purpose: PackagePurpose::Standard,
                compression: CompressionKind::Lz4Frame,
                flags: 7,
                metadata: Some(metadata.clone()),
                integrity: IntegrityKind::Sha256,
            },
        )
        .expect("package should encode");

        let package = DhbinReader::read_package_default(&bytes).expect("package should load");
        assert_eq!(package.header.package_id, *b"CONF");
        assert_eq!(package.metadata, Some(metadata));
        assert!(package.integrity_verified);
    }

    #[test]
    fn checksum_mismatch_is_reported_for_full_profile() {
        let mut bytes = DhbinWriter::write_payload(
            &vec!["alpha".to_string()],
            &DhbinWriteOptions {
                package_id: *b"CONF",
                purpose: PackagePurpose::Standard,
                compression: CompressionKind::None,
                flags: 0,
                metadata: None,
                integrity: IntegrityKind::Sha256,
            },
        )
        .expect("package should encode");

        let last = bytes.len() - 1;
        bytes[last] ^= 0x01;
        let error = DhbinReader::read_package_default(&bytes).expect_err("checksum should fail");
        assert!(matches!(error, DhbinDecodeError::ChecksumMismatch));
    }

    #[test]
    fn metadata_section_is_optional() {
        let bytes = DhbinWriter::write_payload(
            &42u32,
            &DhbinWriteOptions {
                package_id: *b"CONF",
                purpose: PackagePurpose::Standard,
                compression: CompressionKind::None,
                flags: 0,
                metadata: None,
                integrity: IntegrityKind::None,
            },
        )
        .expect("package should encode");

        let package = DhbinReader::read_package_default(&bytes).expect("package should load");
        assert_eq!(package.metadata, None);
        let payload: u32 = DhbinReader::decode_payload_with_options(
            &bytes,
            &DhbinReadOptions {
                verify_integrity: false,
                load_metadata: true,
            },
        )
        .expect("payload should decode");
        assert_eq!(payload, 42);
    }

    #[test]
    fn header_and_section_table_can_be_read_separately() {
        let metadata = rmp_serde::to_vec(&"meta").unwrap();
        let bytes = DhbinWriter::write_payload(
            &"payload",
            &DhbinWriteOptions {
                package_id: *b"FDEF",
                purpose: PackagePurpose::Embedded,
                compression: CompressionKind::None,
                flags: 5,
                metadata: Some(metadata),
                integrity: IntegrityKind::Sha256,
            },
        )
        .expect("package should encode");

        let header = DhbinReader::read_header(&bytes).expect("header should load");
        assert_eq!(
            header,
            PackageHeader {
                wire_version: 2,
                package_id: *b"FDEF",
                purpose: PackagePurpose::Embedded,
                compression: CompressionKind::None,
                flags: 5,
                section_count: 3,
                section_table_offset: 28,
                section_table_len: 72,
            }
        );

        let sections = DhbinReader::read_sections(&bytes, &header).expect("sections should load");
        assert_eq!(sections[0].kind, SectionKind::Payload);
        assert_eq!(sections[1].kind, SectionKind::Metadata);
        assert_eq!(sections[1].format, METADATA_FORMAT_MESSAGEPACK);
        assert_eq!(sections[2].kind, SectionKind::Integrity);
    }
}
