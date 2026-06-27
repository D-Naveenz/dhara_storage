use thiserror::Error;

/// Errors returned while encoding or decoding definition packages.
#[derive(Debug, Error)]
pub enum DefinitionPackageError {
    /// The buffer does not carry the expected FlatBuffers `FDEF` identifier.
    #[error("definition package does not use the expected FDEF FlatBuffers identifier")]
    InvalidIdentifier,

    /// FlatBuffers verification failed.
    #[error("definition package FlatBuffer is invalid: {0}")]
    InvalidFlatbuffer(#[from] flatbuffers::InvalidFlatbuffer),

    /// The bundled package could not be loaded.
    #[error("failed to load bundled definition package: {message}")]
    BundledLoad {
        /// Decode or validation failure message.
        message: String,
    },
}
