use std::io::{Cursor, Read, Write};

use crate::config::{NbtReadConfig, ParseMode};
use crate::core::{read_payload_with_config, write_payload};
use crate::encoding::Encoding;
use crate::error::{Error, Result};
use crate::limits::NbtLimits;
use crate::tag::{Tag, TagType};

fn attach_context<T>(
    op: &'static str,
    offset: usize,
    field: Option<&'static str>,
    result: Result<T>,
) -> Result<T> {
    result.map_err(|error| error.with_context(op, offset, field))
}

pub const BEDROCK_FILE_HEADER_MAGIC: u32 = 8;

#[derive(Debug, Clone, PartialEq)]
pub struct RootTag {
    pub name: String,
    pub payload: Tag,
}

impl RootTag {
    pub fn new(name: impl Into<String>, payload: Tag) -> Self {
        Self {
            name: name.into(),
            payload,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderReadMode {
    NoHeader,
    BedrockFileHeader,
    LevelDatHeader,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderWriteMode {
    NoHeader,
    BedrockFileHeader,
    LevelDatHeader { storage_version: u32 },
}

pub fn read_tag<E: Encoding, R: Read>(reader: &mut R) -> Result<RootTag> {
    read_tag_with_limits::<E, _>(reader, &NbtLimits::default())
}

pub fn read_tag_with_limits<E: Encoding, R: Read>(
    reader: &mut R,
    limits: &NbtLimits,
) -> Result<RootTag> {
    read_tag_with_config::<E, _>(reader, &NbtReadConfig::strict(*limits))
}

pub fn read_tag_with_config<E: Encoding, R: Read>(
    reader: &mut R,
    config: &NbtReadConfig,
) -> Result<RootTag> {
    let mut id = [0u8; 1];
    attach_context(
        "read_exact",
        0,
        Some("root_tag_type"),
        reader.read_exact(&mut id).map_err(Error::from),
    )?;
    let tag_type = attach_context(
        "decode_tag_type",
        0,
        Some("root_tag_type"),
        TagType::try_from(id[0]),
    )?;
    if !is_valid_root_tag_type(tag_type, config.parse_mode) {
        return Err(Error::InvalidRoot { id: id[0] }.with_context(
            "validate_root_tag_type",
            0,
            Some("root_tag_type"),
        ));
    }

    let name = read_string::<E, _>(reader, "root_name", &config.limits, 1)?;
    let payload = read_payload_with_config::<E, _>(reader, tag_type, config)
        .map_err(|error| error.with_context("read_root_payload", 1, Some("root_payload")))?;
    Ok(RootTag { name, payload })
}

pub fn write_tag<E: Encoding, W: Write>(writer: &mut W, root: &RootTag) -> Result<()> {
    let tag_type = root.payload.tag_type();
    if !is_valid_root_tag_type(tag_type, ParseMode::Strict) {
        return Err(Error::InvalidRoot { id: tag_type.id() }.with_context(
            "validate_root_tag_type",
            0,
            Some("root_tag_type"),
        ));
    }
    writer.write_all(&[tag_type.id()])?;
    write_string::<E, _>(writer, &root.name)?;
    write_payload::<E, _>(writer, &root.payload)
}

pub fn read_with_header_mode<E: Encoding, R: Read>(
    reader: &mut R,
    mode: HeaderReadMode,
) -> Result<RootTag> {
    read_with_header_mode_with_limits::<E, _>(reader, mode, &NbtLimits::default())
}

pub fn read_with_header_mode_with_limits<E: Encoding, R: Read>(
    reader: &mut R,
    mode: HeaderReadMode,
    limits: &NbtLimits,
) -> Result<RootTag> {
    read_with_header_mode_with_config::<E, _>(reader, mode, &NbtReadConfig::strict(*limits))
}

pub fn read_with_header_mode_with_config<E: Encoding, R: Read>(
    reader: &mut R,
    mode: HeaderReadMode,
    config: &NbtReadConfig,
) -> Result<RootTag> {
    match mode {
        HeaderReadMode::NoHeader => read_tag_with_config::<E, _>(reader, config),
        HeaderReadMode::BedrockFileHeader => {
            let magic = read_u32_le(reader, 0, "bedrock_header_magic")?;
            if magic != BEDROCK_FILE_HEADER_MAGIC {
                return Err(Error::InvalidHeader {
                    detail: "bedrock_header_magic_mismatch",
                    expected: Some(BEDROCK_FILE_HEADER_MAGIC),
                    actual: Some(magic),
                }
                .with_context(
                    "validate_header_magic",
                    0,
                    Some("bedrock_header_magic"),
                ));
            }
            let payload_len = read_u32_le(reader, 4, "bedrock_header_payload_length")? as usize;
            attach_context(
                "validate_size",
                4,
                Some("header_payload_length"),
                ensure_within_limit(
                    "header_payload_length",
                    payload_len,
                    config.limits.max_read_bytes,
                ),
            )?;
            read_root_from_len_prefixed_payload::<E, _>(reader, payload_len, config)
        }
        HeaderReadMode::LevelDatHeader => {
            let _storage_version = read_u32_le(reader, 0, "leveldat_storage_version")?;
            let payload_len = read_u32_le(reader, 4, "leveldat_payload_length")? as usize;
            attach_context(
                "validate_size",
                4,
                Some("header_payload_length"),
                ensure_within_limit(
                    "header_payload_length",
                    payload_len,
                    config.limits.max_read_bytes,
                ),
            )?;
            read_root_from_len_prefixed_payload::<E, _>(reader, payload_len, config)
        }
    }
}

pub fn write_with_header_mode<E: Encoding, W: Write>(
    writer: &mut W,
    root: &RootTag,
    mode: HeaderWriteMode,
) -> Result<()> {
    match mode {
        HeaderWriteMode::NoHeader => write_tag::<E, _>(writer, root),
        HeaderWriteMode::BedrockFileHeader => {
            let payload = encode_root_payload::<E>(root)?;
            write_u32_le(writer, BEDROCK_FILE_HEADER_MAGIC)?;
            write_u32_le(writer, payload_len_u32(payload.len())?)?;
            writer.write_all(&payload)?;
            Ok(())
        }
        HeaderWriteMode::LevelDatHeader { storage_version } => {
            let payload = encode_root_payload::<E>(root)?;
            write_u32_le(writer, storage_version)?;
            write_u32_le(writer, payload_len_u32(payload.len())?)?;
            writer.write_all(&payload)?;
            Ok(())
        }
    }
}

fn read_root_from_len_prefixed_payload<E: Encoding, R: Read>(
    reader: &mut R,
    payload_len: usize,
    config: &NbtReadConfig,
) -> Result<RootTag> {
    let mut payload = vec![0u8; payload_len];
    attach_context(
        "read_exact",
        8,
        Some("header_payload"),
        reader.read_exact(&mut payload).map_err(Error::from),
    )?;

    let mut cursor = Cursor::new(payload.as_slice());
    let root = read_tag_with_config::<E, _>(&mut cursor, config)
        .map_err(|error| error.with_context("read_header_payload_root", 8, Some("root_payload")))?;
    let consumed = cursor.position() as usize;
    let unread = payload_len.saturating_sub(consumed);
    if unread != 0 {
        return Err(Error::TrailingPayloadBytes { unread }.with_context(
            "validate_payload_consumed",
            8 + consumed,
            Some("header_payload"),
        ));
    }
    Ok(root)
}

fn encode_root_payload<E: Encoding>(root: &RootTag) -> Result<Vec<u8>> {
    let mut payload = Vec::new();
    write_tag::<E, _>(&mut payload, root)?;
    Ok(payload)
}

fn payload_len_u32(payload_len: usize) -> Result<u32> {
    u32::try_from(payload_len).map_err(|_| Error::LengthOverflow {
        field: "header_payload_length",
        max: u32::MAX as usize,
        actual: payload_len,
    })
}

fn read_string<E: Encoding, R: Read>(
    reader: &mut R,
    field: &'static str,
    limits: &NbtLimits,
    offset: usize,
) -> Result<String> {
    let len = attach_context(
        "read_string_len",
        offset,
        Some(field),
        E::read_string_len(reader),
    )?;
    attach_context(
        "validate_size",
        offset,
        Some("root_name_length"),
        ensure_within_limit("root_name_length", len, limits.max_string_len),
    )?;
    let mut bytes = vec![0u8; len];
    attach_context(
        "read_exact",
        offset,
        Some(field),
        reader.read_exact(&mut bytes).map_err(Error::from),
    )?;
    let decode_res = String::from_utf8(bytes).map_err(|_| Error::InvalidUtf8 { field });
    attach_context("decode_utf8", offset, Some(field), decode_res)
}

fn write_string<E: Encoding, W: Write>(writer: &mut W, value: &str) -> Result<()> {
    E::write_string_len(writer, value.len())?;
    writer.write_all(value.as_bytes())?;
    Ok(())
}

fn read_u32_le<R: Read>(reader: &mut R, offset: usize, field: &'static str) -> Result<u32> {
    let mut bytes = [0u8; 4];
    attach_context(
        "read_exact",
        offset,
        Some(field),
        reader.read_exact(&mut bytes).map_err(Error::from),
    )?;
    Ok(u32::from_le_bytes(bytes))
}

fn write_u32_le<W: Write>(writer: &mut W, value: u32) -> Result<()> {
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

fn ensure_within_limit(field: &'static str, actual: usize, max: usize) -> Result<()> {
    if actual > max {
        return Err(Error::SizeExceeded { field, max, actual });
    }
    Ok(())
}

fn is_valid_root_tag_type(tag_type: TagType, parse_mode: ParseMode) -> bool {
    match parse_mode {
        ParseMode::Strict => matches!(tag_type, TagType::Compound | TagType::List),
        ParseMode::Compatible => tag_type != TagType::End,
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::io::ErrorKind;

    use indexmap::IndexMap;

    use crate::config::NbtReadConfig;
    use crate::encoding::{BigEndian, LittleEndian, NetworkLittleEndian};
    use crate::limits::NbtLimits;
    use crate::tag::ListTag;

    use super::*;

    fn sample_root(name: &str) -> RootTag {
        let mut map = IndexMap::new();
        map.insert("health".to_string(), Tag::Int(20));
        map.insert("name".to_string(), Tag::String("Steve".to_string()));
        RootTag::new(name, Tag::Compound(map))
    }

    #[test]
    fn root_roundtrip_preserves_name_be() {
        let root = sample_root("PlayerData");
        let mut out = Vec::new();
        write_tag::<BigEndian, _>(&mut out, &root).unwrap();

        let mut input = Cursor::new(out);
        let decoded = read_tag::<BigEndian, _>(&mut input).unwrap();
        assert_eq!(decoded, root);
    }

    #[test]
    fn root_roundtrip_empty_name_nle() {
        let root = sample_root("");
        let mut out = Vec::new();
        write_tag::<NetworkLittleEndian, _>(&mut out, &root).unwrap();

        let mut input = Cursor::new(out);
        let decoded = read_tag::<NetworkLittleEndian, _>(&mut input).unwrap();
        assert_eq!(decoded.name, "");
        assert_eq!(decoded.payload, root.payload);
    }

    #[test]
    fn root_rejects_tag_end_id() {
        let mut input = Cursor::new(vec![TagType::End.id(), 0x00, 0x00]);
        let err = read_tag::<LittleEndian, _>(&mut input);
        let err = err.unwrap_err();
        assert!(matches!(err.innermost(), Error::InvalidRoot { id: 0 }));
    }

    #[test]
    fn root_rejects_primitive_tag_id() {
        let mut input = Cursor::new(vec![TagType::Int.id(), 0x00, 0x00, 0x00, 0x00, 0x00, 0x2A]);
        let err = read_tag::<BigEndian, _>(&mut input).unwrap_err();
        assert!(matches!(
            err.innermost(),
            Error::InvalidRoot { id } if *id == TagType::Int.id()
        ));
        assert!(err.has_context("validate_root_tag_type", Some("root_tag_type")));
    }

    #[test]
    fn compatible_mode_accepts_primitive_root_tag_id() {
        let mut input = Cursor::new(vec![TagType::Int.id(), 0x00, 0x00, 0x00, 0x00, 0x00, 0x2A]);
        let config = NbtReadConfig::compatible(NbtLimits::default());
        let decoded = read_tag_with_config::<BigEndian, _>(&mut input, &config).unwrap();
        assert_eq!(decoded.name, "");
        assert_eq!(decoded.payload, Tag::Int(42));
    }

    #[test]
    fn write_tag_rejects_primitive_root_payload() {
        let root = RootTag::new("bad", Tag::Int(7));
        let mut out = Vec::new();
        let err = write_tag::<BigEndian, _>(&mut out, &root).unwrap_err();
        assert!(matches!(
            err.innermost(),
            Error::InvalidRoot { id } if *id == TagType::Int.id()
        ));
    }

    #[test]
    fn root_list_roundtrip_is_allowed() {
        let list = ListTag::new(TagType::Int, vec![Tag::Int(1), Tag::Int(2)]).unwrap();
        let root = RootTag::new("list_root", Tag::List(list));
        let mut out = Vec::new();
        write_tag::<BigEndian, _>(&mut out, &root).unwrap();

        let mut input = Cursor::new(out);
        let decoded = read_tag::<BigEndian, _>(&mut input).unwrap();
        assert_eq!(decoded, root);
    }

    #[test]
    fn no_header_read_stops_at_root_end_and_leaves_trailing_bytes() {
        let bytes = vec![
            TagType::Compound.id(),
            0x00,
            0x00, // root name len = 0
            0x00, // empty compound end
            0xAB, // trailing byte
        ];

        let mut input = Cursor::new(bytes);
        let root = read_tag::<BigEndian, _>(&mut input).unwrap();
        assert!(matches!(root.payload, Tag::Compound(_)));
        assert_eq!(input.position(), 4);
    }

    #[test]
    fn no_header_mode_rejects_bedrock_header_stream_in_strict_mode() {
        let root = sample_root("BedrockRoot");
        let mut out = Vec::new();
        write_with_header_mode::<LittleEndian, _>(
            &mut out,
            &root,
            HeaderWriteMode::BedrockFileHeader,
        )
        .unwrap();

        let mut input = Cursor::new(out);
        let err = read_with_header_mode::<LittleEndian, _>(&mut input, HeaderReadMode::NoHeader)
            .unwrap_err();
        assert!(matches!(
            err.innermost(),
            Error::InvalidRoot { id } if *id == TagType::String.id()
        ));
    }

    #[test]
    fn no_header_mode_rejects_leveldat_header_stream_in_strict_mode() {
        let root = sample_root("LevelDatRoot");
        let mut out = Vec::new();
        write_with_header_mode::<LittleEndian, _>(
            &mut out,
            &root,
            HeaderWriteMode::LevelDatHeader {
                storage_version: 11,
            },
        )
        .unwrap();

        let mut input = Cursor::new(out);
        let err = read_with_header_mode::<LittleEndian, _>(&mut input, HeaderReadMode::NoHeader)
            .unwrap_err();
        assert!(matches!(
            err.innermost(),
            Error::InvalidRoot { id } if *id == TagType::IntArray.id()
        ));
    }

    #[test]
    fn bedrock_header_roundtrip() {
        let root = sample_root("BedrockRoot");
        let mut out = Vec::new();
        write_with_header_mode::<LittleEndian, _>(
            &mut out,
            &root,
            HeaderWriteMode::BedrockFileHeader,
        )
        .unwrap();

        let mut input = Cursor::new(out);
        let decoded =
            read_with_header_mode::<LittleEndian, _>(&mut input, HeaderReadMode::BedrockFileHeader)
                .unwrap();
        assert_eq!(decoded, root);
    }

    #[test]
    fn bedrock_header_rejects_invalid_magic() {
        let root = sample_root("BedrockRoot");
        let mut payload = Vec::new();
        write_tag::<LittleEndian, _>(&mut payload, &root).unwrap();

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&7u32.to_le_bytes());
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&payload);

        let mut input = Cursor::new(bytes);
        let err =
            read_with_header_mode::<LittleEndian, _>(&mut input, HeaderReadMode::BedrockFileHeader);
        let err = err.unwrap_err();
        assert!(matches!(
            err.innermost(),
            Error::InvalidHeader {
                detail: "bedrock_header_magic_mismatch",
                expected: Some(BEDROCK_FILE_HEADER_MAGIC),
                actual: Some(7)
            }
        ));
    }

    #[test]
    fn bedrock_header_rejects_no_header_stream() {
        let root = sample_root("NoHeaderRoot");
        let mut out = Vec::new();
        write_tag::<LittleEndian, _>(&mut out, &root).unwrap();

        let mut input = Cursor::new(out);
        let err =
            read_with_header_mode::<LittleEndian, _>(&mut input, HeaderReadMode::BedrockFileHeader)
                .unwrap_err();
        assert!(matches!(
            err.innermost(),
            Error::InvalidHeader {
                detail: "bedrock_header_magic_mismatch",
                expected: Some(BEDROCK_FILE_HEADER_MAGIC),
                ..
            }
        ));
    }

    #[test]
    fn bedrock_header_rejects_leveldat_stream_when_storage_version_is_not_magic() {
        let root = sample_root("LevelDatRoot");
        let mut out = Vec::new();
        write_with_header_mode::<LittleEndian, _>(
            &mut out,
            &root,
            HeaderWriteMode::LevelDatHeader {
                storage_version: 11,
            },
        )
        .unwrap();

        let mut input = Cursor::new(out);
        let err =
            read_with_header_mode::<LittleEndian, _>(&mut input, HeaderReadMode::BedrockFileHeader)
                .unwrap_err();
        assert!(matches!(
            err.innermost(),
            Error::InvalidHeader {
                detail: "bedrock_header_magic_mismatch",
                expected: Some(BEDROCK_FILE_HEADER_MAGIC),
                actual: Some(11)
            }
        ));
    }

    #[test]
    fn leveldat_header_accepts_non_eight_storage_version() {
        let root = sample_root("LevelDatRoot");
        let mut out = Vec::new();
        write_with_header_mode::<LittleEndian, _>(
            &mut out,
            &root,
            HeaderWriteMode::LevelDatHeader {
                storage_version: 11,
            },
        )
        .unwrap();

        let mut input = Cursor::new(out);
        let decoded =
            read_with_header_mode::<LittleEndian, _>(&mut input, HeaderReadMode::LevelDatHeader)
                .unwrap();
        assert_eq!(decoded, root);
    }

    #[test]
    fn leveldat_header_can_read_bedrock_stream_due_to_layout_compatibility() {
        let root = sample_root("BedrockRoot");
        let mut out = Vec::new();
        write_with_header_mode::<LittleEndian, _>(
            &mut out,
            &root,
            HeaderWriteMode::BedrockFileHeader,
        )
        .unwrap();

        let mut input = Cursor::new(out);
        let decoded =
            read_with_header_mode::<LittleEndian, _>(&mut input, HeaderReadMode::LevelDatHeader)
                .unwrap();
        assert_eq!(decoded, root);
    }

    #[test]
    fn header_payload_trailing_bytes_are_rejected() {
        let root = sample_root("BedrockRoot");
        let mut payload = Vec::new();
        write_tag::<LittleEndian, _>(&mut payload, &root).unwrap();
        payload.extend_from_slice(&[0xAA, 0xBB, 0xCC]);

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&BEDROCK_FILE_HEADER_MAGIC.to_le_bytes());
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&payload);

        let mut input = Cursor::new(bytes);
        let err =
            read_with_header_mode::<LittleEndian, _>(&mut input, HeaderReadMode::BedrockFileHeader);
        let err = err.unwrap_err();
        assert!(matches!(
            err.innermost(),
            Error::TrailingPayloadBytes { unread: 3 }
        ));
    }

    #[test]
    fn bedrock_header_rejects_payload_size_larger_than_stream() {
        let root = sample_root("BedrockRoot");
        let mut payload = Vec::new();
        write_tag::<LittleEndian, _>(&mut payload, &root).unwrap();

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&BEDROCK_FILE_HEADER_MAGIC.to_le_bytes());
        bytes.extend_from_slice(&((payload.len() as u32) + 5).to_le_bytes());
        bytes.extend_from_slice(&payload);

        let mut input = Cursor::new(bytes);
        let err =
            read_with_header_mode::<LittleEndian, _>(&mut input, HeaderReadMode::BedrockFileHeader)
                .unwrap_err();
        assert!(matches!(
            err.innermost(),
            Error::Io(io_error) if io_error.kind() == ErrorKind::UnexpectedEof
        ));
    }

    #[test]
    fn bedrock_header_payload_length_over_limit_is_rejected_before_allocation() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&BEDROCK_FILE_HEADER_MAGIC.to_le_bytes());
        bytes.extend_from_slice(&(1024u32).to_le_bytes());

        let mut input = Cursor::new(bytes);
        let limits = NbtLimits::default().with_max_read_bytes(64);
        let err = read_with_header_mode_with_limits::<LittleEndian, _>(
            &mut input,
            HeaderReadMode::BedrockFileHeader,
            &limits,
        )
        .unwrap_err();
        assert!(matches!(
            err.innermost(),
            Error::SizeExceeded {
                field: "header_payload_length",
                max: 64,
                actual: 1024
            }
        ));
    }

    #[test]
    fn bedrock_header_rejects_payload_size_smaller_than_payload() {
        let root = sample_root("BedrockRoot");
        let mut payload = Vec::new();
        write_tag::<LittleEndian, _>(&mut payload, &root).unwrap();
        assert!(!payload.is_empty());

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&BEDROCK_FILE_HEADER_MAGIC.to_le_bytes());
        bytes.extend_from_slice(&((payload.len() as u32) - 1).to_le_bytes());
        bytes.extend_from_slice(&payload);

        let mut input = Cursor::new(bytes);
        let err =
            read_with_header_mode::<LittleEndian, _>(&mut input, HeaderReadMode::BedrockFileHeader)
                .unwrap_err();
        assert!(matches!(
            err.innermost(),
            Error::Io(io_error) if io_error.kind() == ErrorKind::UnexpectedEof
        ));
    }

    #[test]
    fn leveldat_header_payload_length_over_limit_is_rejected_before_allocation() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(11u32).to_le_bytes());
        bytes.extend_from_slice(&(1024u32).to_le_bytes());

        let mut input = Cursor::new(bytes);
        let limits = NbtLimits::default().with_max_read_bytes(64);
        let err = read_with_header_mode_with_limits::<LittleEndian, _>(
            &mut input,
            HeaderReadMode::LevelDatHeader,
            &limits,
        )
        .unwrap_err();
        assert!(matches!(
            err.innermost(),
            Error::SizeExceeded {
                field: "header_payload_length",
                max: 64,
                actual: 1024
            }
        ));
    }
}
