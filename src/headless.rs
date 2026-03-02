use std::io::{Cursor, Read, Write};

use crate::config::NbtReadConfig;
use crate::core::{read_payload, read_payload_with_config, write_payload};
use crate::encoding::Encoding;
use crate::error::{Error, Result};
use crate::limits::NbtLimits;
use crate::tag::{Tag, TagType};

/// Reads a headless NBT value payload using an externally known tag type.
///
/// This is the canonical low-level API for protocol fields where the type is
/// provided by surrounding packet metadata.
pub fn read_value<E: Encoding, R: Read>(reader: &mut R, tag_type: TagType) -> Result<Tag> {
    read_payload::<E, _>(reader, tag_type)
}

pub fn read_value_with_limits<E: Encoding, R: Read>(
    reader: &mut R,
    tag_type: TagType,
    limits: &NbtLimits,
) -> Result<Tag> {
    read_payload_with_config::<E, _>(reader, tag_type, &NbtReadConfig::strict(*limits))
}

pub fn read_value_with_config<E: Encoding, R: Read>(
    reader: &mut R,
    tag_type: TagType,
    config: &NbtReadConfig,
) -> Result<Tag> {
    read_payload_with_config::<E, _>(reader, tag_type, config)
}

/// Writes a headless NBT value payload (no tag id, no name).
pub fn write_value<E: Encoding, W: Write>(writer: &mut W, value: &Tag) -> Result<()> {
    write_payload::<E, _>(writer, value)
}

/// Alias kept for API ergonomics in packet codec code.
pub fn read_headless<E: Encoding, R: Read>(reader: &mut R, tag_type: TagType) -> Result<Tag> {
    read_value::<E, _>(reader, tag_type)
}

pub fn read_headless_with_limits<E: Encoding, R: Read>(
    reader: &mut R,
    tag_type: TagType,
    limits: &NbtLimits,
) -> Result<Tag> {
    read_value_with_limits::<E, _>(reader, tag_type, limits)
}

pub fn read_headless_with_config<E: Encoding, R: Read>(
    reader: &mut R,
    tag_type: TagType,
    config: &NbtReadConfig,
) -> Result<Tag> {
    read_value_with_config::<E, _>(reader, tag_type, config)
}

/// Alias kept for API ergonomics in packet codec code.
pub fn write_headless<E: Encoding, W: Write>(writer: &mut W, value: &Tag) -> Result<()> {
    write_value::<E, _>(writer, value)
}

/// Reads a headless value when only the type id byte is available.
pub fn read_headless_by_id<E: Encoding, R: Read>(reader: &mut R, tag_type_id: u8) -> Result<Tag> {
    let tag_type = TagType::try_from(tag_type_id)
        .map_err(|error| error.with_context("decode_tag_type", 0, Some("tag_type_id")))?;
    read_headless::<E, _>(reader, tag_type)
}

pub fn read_headless_by_id_with_limits<E: Encoding, R: Read>(
    reader: &mut R,
    tag_type_id: u8,
    limits: &NbtLimits,
) -> Result<Tag> {
    let tag_type = TagType::try_from(tag_type_id)
        .map_err(|error| error.with_context("decode_tag_type", 0, Some("tag_type_id")))?;
    read_headless_with_limits::<E, _>(reader, tag_type, limits)
}

pub fn read_headless_by_id_with_config<E: Encoding, R: Read>(
    reader: &mut R,
    tag_type_id: u8,
    config: &NbtReadConfig,
) -> Result<Tag> {
    let tag_type = TagType::try_from(tag_type_id)
        .map_err(|error| error.with_context("decode_tag_type", 0, Some("tag_type_id")))?;
    read_headless_with_config::<E, _>(reader, tag_type, config)
}

/// Reads a value where a type id byte is prefixed before headless payload.
pub fn read_headless_prefixed<E: Encoding, R: Read>(reader: &mut R) -> Result<Tag> {
    let mut id = [0u8; 1];
    reader
        .read_exact(&mut id)
        .map_err(Error::from)
        .map_err(|error| error.with_context("read_exact", 0, Some("tag_type_id")))?;
    read_headless_by_id::<E, _>(reader, id[0])
}

pub fn read_headless_prefixed_with_limits<E: Encoding, R: Read>(
    reader: &mut R,
    limits: &NbtLimits,
) -> Result<Tag> {
    let mut id = [0u8; 1];
    reader
        .read_exact(&mut id)
        .map_err(Error::from)
        .map_err(|error| error.with_context("read_exact", 0, Some("tag_type_id")))?;
    read_headless_by_id_with_limits::<E, _>(reader, id[0], limits)
}

pub fn read_headless_prefixed_with_config<E: Encoding, R: Read>(
    reader: &mut R,
    config: &NbtReadConfig,
) -> Result<Tag> {
    let mut id = [0u8; 1];
    reader
        .read_exact(&mut id)
        .map_err(Error::from)
        .map_err(|error| error.with_context("read_exact", 0, Some("tag_type_id")))?;
    read_headless_by_id_with_config::<E, _>(reader, id[0], config)
}

/// Writes a value as `type_id + payload` without root-name.
pub fn write_headless_prefixed<E: Encoding, W: Write>(writer: &mut W, value: &Tag) -> Result<()> {
    if matches!(value, Tag::End) {
        return Err(Error::UnexpectedEndTagPayload);
    }
    writer.write_all(&[value.tag_type().id()])?;
    write_headless::<E, _>(writer, value)
}

/// Convenience decode from bytes for headless payload.
pub fn from_headless_bytes<E: Encoding>(bytes: &[u8], tag_type: TagType) -> Result<Tag> {
    let mut cursor = Cursor::new(bytes);
    read_headless::<E, _>(&mut cursor, tag_type)
}

/// Convenience encode to bytes for headless payload.
pub fn to_headless_bytes<E: Encoding>(value: &Tag) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    write_headless::<E, _>(&mut out, value)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use indexmap::IndexMap;

    use crate::encoding::{BigEndian, NetworkLittleEndian};
    use crate::error::Error;
    use crate::tag::ListTag;

    use super::*;

    #[test]
    fn value_aliases_roundtrip_be() {
        let value = Tag::String("hello".to_string());
        let mut out = Vec::new();
        write_value::<BigEndian, _>(&mut out, &value).unwrap();

        let mut input = Cursor::new(out);
        let decoded = read_headless::<BigEndian, _>(&mut input, TagType::String).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn prefixed_roundtrip_nle_compound() {
        let mut map = IndexMap::new();
        map.insert("score".to_string(), Tag::Int(42));
        map.insert(
            "flags".to_string(),
            Tag::List(ListTag::new(TagType::Byte, vec![Tag::Byte(1), Tag::Byte(0)]).unwrap()),
        );
        let value = Tag::Compound(map);

        let mut out = Vec::new();
        write_headless_prefixed::<NetworkLittleEndian, _>(&mut out, &value).unwrap();

        let mut input = Cursor::new(out);
        let decoded = read_headless_prefixed::<NetworkLittleEndian, _>(&mut input).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn headless_by_id_rejects_unknown_tag_id() {
        let mut input = Cursor::new(Vec::<u8>::new());
        let err = read_headless_by_id::<BigEndian, _>(&mut input, 99);
        let err = err.unwrap_err();
        assert!(matches!(err.innermost(), Error::UnknownTag { id: 99 }));
    }

    #[test]
    fn headless_prefixed_rejects_tag_end_payload() {
        let mut input = Cursor::new(vec![TagType::End.id()]);
        let err = read_headless_prefixed::<BigEndian, _>(&mut input);
        let err = err.unwrap_err();
        assert!(matches!(err.innermost(), Error::UnexpectedEndTagPayload));
    }

    #[test]
    fn to_from_headless_bytes_helpers_work() {
        let value = Tag::IntArray(vec![1, 2, 3, 4]);
        let bytes = to_headless_bytes::<BigEndian>(&value).unwrap();
        let decoded = from_headless_bytes::<BigEndian>(&bytes, TagType::IntArray).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn to_from_headless_bytes_preserves_long_array_variant() {
        let value = Tag::LongArray(vec![-5, 0, 7, i64::MIN, i64::MAX]);
        let bytes = to_headless_bytes::<BigEndian>(&value).unwrap();
        let decoded = from_headless_bytes::<BigEndian>(&bytes, TagType::LongArray).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn write_headless_prefixed_rejects_end_tag_without_writing_bytes() {
        let mut out = Vec::new();
        let err = write_headless_prefixed::<BigEndian, _>(&mut out, &Tag::End).unwrap_err();
        assert!(matches!(err, Error::UnexpectedEndTagPayload));
        assert!(out.is_empty());
    }
}
