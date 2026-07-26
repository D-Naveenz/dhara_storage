//! DSFD on-disk format: FlatBuffers payload, container framing, and XML footer.

mod codec;
mod container;
mod metadata_xml;

pub use container::{
    decode_definition_package, encode_definition_package, root_definition_package,
};
