use thiserror::Error;

/// Errors returned while encoding or decoding definition packages.
#[derive(Debug, Error)]
pub enum DefinitionPackageError {
    /// The file does not begin with the expected `DSFD` magic.
    #[error("definition package does not begin with the expected DSFD magic")]
    InvalidMagic,

    /// The file does not end with the expected `DSFD` magic.
    #[error("definition package does not end with the expected DSFD magic")]
    InvalidEndMagic,

    /// The container format version is not supported.
    #[error("unsupported DSFD container format version: {version}")]
    UnsupportedFormatVersion {
        /// Unsupported format version value.
        version: u16,
    },

    /// The container lengths are inconsistent with the file size.
    #[error("definition package container lengths are invalid: {message}")]
    InvalidContainerLayout {
        /// Layout validation failure message.
        message: String,
    },

    /// The FlatBuffers payload does not carry the expected `DSFD` identifier.
    #[error("definition package payload does not use the expected DSFD FlatBuffers identifier")]
    InvalidIdentifier,

    /// FlatBuffers verification failed.
    #[error("definition package FlatBuffer is invalid: {0}")]
    InvalidFlatbuffer(#[from] flatbuffers::InvalidFlatbuffer),

    /// XML metadata could not be parsed.
    #[error("definition package metadata XML is invalid: {message}")]
    InvalidMetadataXml {
        /// XML parse or validation failure message.
        message: String,
    },

    /// XML metadata failed semantic validation.
    #[error("definition package metadata is invalid: {message}")]
    InvalidMetadata {
        /// Semantic validation failure message.
        message: String,
    },

    /// The bundled package could not be loaded.
    #[error("failed to load bundled definition package: {message}")]
    BundledLoad {
        /// Decode or validation failure message.
        message: String,
    },
}
