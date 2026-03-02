use std::io::{Read, Write};

use crate::config::{NbtReadConfig, ParseMode};
use crate::encoding::Encoding;
use crate::error::{Error, Result};
use crate::limits::NbtLimits;
use crate::tag::{CompoundTag, ListTag, Tag, TagType};

fn attach_context<T>(
    op: &'static str,
    offset: usize,
    field: Option<&'static str>,
    result: Result<T>,
) -> Result<T> {
    result.map_err(|error| error.with_context(op, offset, field))
}

pub fn read_payload<E: Encoding, R: Read>(reader: &mut R, tag_type: TagType) -> Result<Tag> {
    read_payload_with_limits::<E, _>(reader, tag_type, &NbtLimits::default())
}

pub fn read_payload_with_limits<E: Encoding, R: Read>(
    reader: &mut R,
    tag_type: TagType,
    limits: &NbtLimits,
) -> Result<Tag> {
    read_payload_with_config::<E, _>(reader, tag_type, &NbtReadConfig::strict(*limits))
}

pub fn read_payload_with_config<E: Encoding, R: Read>(
    reader: &mut R,
    tag_type: TagType,
    config: &NbtReadConfig,
) -> Result<Tag> {
    let mut limited = LimitedReader::new(reader, config.limits.max_read_bytes);
    let result = read_payload_inner::<E, _>(&mut limited, tag_type, config, 1);
    attach_context(
        "read_payload_with_config",
        limited.offset(),
        Some("payload"),
        result,
    )
}

fn read_payload_inner<E: Encoding, R: Read>(
    reader: &mut LimitedReader<R>,
    tag_type: TagType,
    config: &NbtReadConfig,
    depth: usize,
) -> Result<Tag> {
    if depth > config.limits.max_depth {
        return Err(Error::DepthExceeded {
            depth,
            max_depth: config.limits.max_depth,
        }
        .with_context("check_depth", reader.offset(), Some("max_depth")));
    }

    match tag_type {
        TagType::End => Err(Error::UnexpectedEndTagPayload),
        TagType::Byte => Ok(Tag::Byte(read_i8(reader)?)),
        TagType::Short => {
            let offset = reader.offset();
            let value = E::read_i16(reader);
            Ok(Tag::Short(attach_context(
                "read_i16",
                offset,
                Some("short_payload"),
                value,
            )?))
        }
        TagType::Int => {
            let offset = reader.offset();
            let value = E::read_i32(reader);
            Ok(Tag::Int(attach_context(
                "read_i32",
                offset,
                Some("int_payload"),
                value,
            )?))
        }
        TagType::Long => {
            let offset = reader.offset();
            let value = E::read_i64(reader);
            Ok(Tag::Long(attach_context(
                "read_i64",
                offset,
                Some("long_payload"),
                value,
            )?))
        }
        TagType::Float => {
            let offset = reader.offset();
            let value = E::read_f32(reader);
            Ok(Tag::Float(attach_context(
                "read_f32",
                offset,
                Some("float_payload"),
                value,
            )?))
        }
        TagType::Double => {
            let offset = reader.offset();
            let value = E::read_f64(reader);
            Ok(Tag::Double(attach_context(
                "read_f64",
                offset,
                Some("double_payload"),
                value,
            )?))
        }
        TagType::ByteArray => {
            let len = read_len_i32::<E, _>(reader, "byte_array_length")?;
            let limit_offset = reader.offset();
            let limit_res =
                ensure_within_limit("byte_array_length", len, config.limits.max_array_len);
            attach_context(
                "validate_size",
                limit_offset,
                Some("byte_array_length"),
                limit_res,
            )?;
            let offset = reader.offset();
            let budget_res = reader.ensure_can_read("byte_array_bytes", len);
            attach_context(
                "ensure_can_read",
                offset,
                Some("byte_array_bytes"),
                budget_res,
            )?;
            let mut bytes = vec![0u8; len];
            let offset = reader.offset();
            let read_res = reader.read_exact(&mut bytes).map_err(Error::from);
            attach_context("read_exact", offset, Some("byte_array_bytes"), read_res)?;
            Ok(Tag::ByteArray(bytes))
        }
        TagType::String => Ok(Tag::String(read_string::<E, _>(reader, &config.limits)?)),
        TagType::List => {
            let element_type = read_tag_type(reader)?;
            let offset = reader.offset();
            let len_res = E::read_list_len(reader);
            let len = attach_context("read_list_len", offset, Some("list_length"), len_res)?;
            let limit_offset = reader.offset();
            let limit_res = ensure_within_limit("list_length", len, config.limits.max_list_len);
            attach_context(
                "validate_size",
                limit_offset,
                Some("list_length"),
                limit_res,
            )?;
            let effective_len = if element_type == TagType::End && len > 0 {
                match config.parse_mode {
                    ParseMode::Strict => {
                        return Err(Error::InvalidListHeader {
                            element_type_id: element_type.id(),
                            length: len,
                        }
                        .with_context(
                            "validate_list_header",
                            limit_offset,
                            Some("list_length"),
                        ));
                    }
                    ParseMode::Compatible => 0,
                }
            } else {
                len
            };

            let mut elements = Vec::with_capacity(effective_len);
            for _ in 0..effective_len {
                elements.push(read_payload_inner::<E, _>(
                    reader,
                    element_type,
                    config,
                    depth + 1,
                )?);
            }
            Ok(Tag::List(ListTag {
                element_type,
                elements,
            }))
        }
        TagType::Compound => {
            let mut map = CompoundTag::new();
            let mut entry_count = 0usize;
            loop {
                let next_type = read_tag_type(reader)?;
                if next_type == TagType::End {
                    break;
                }
                entry_count += 1;
                let limit_offset = reader.offset();
                let limit_res = ensure_within_limit(
                    "compound_entries",
                    entry_count,
                    config.limits.max_compound_entries,
                );
                attach_context(
                    "validate_size",
                    limit_offset,
                    Some("compound_entries"),
                    limit_res,
                )?;

                let name = read_string::<E, _>(reader, &config.limits)?;
                let value = read_payload_inner::<E, _>(reader, next_type, config, depth + 1)?;
                map.insert(name, value);
            }
            Ok(Tag::Compound(map))
        }
        TagType::IntArray => {
            let len = read_len_i32::<E, _>(reader, "int_array_length")?;
            let limit_offset = reader.offset();
            let limit_res =
                ensure_within_limit("int_array_length", len, config.limits.max_array_len);
            attach_context(
                "validate_size",
                limit_offset,
                Some("int_array_length"),
                limit_res,
            )?;
            let byte_len =
                checked_len_to_bytes(len, std::mem::size_of::<i32>(), "int_array_bytes")?;
            let offset = reader.offset();
            let budget_res = reader.ensure_can_read("int_array_bytes", byte_len);
            attach_context(
                "ensure_can_read",
                offset,
                Some("int_array_bytes"),
                budget_res,
            )?;
            let mut values = Vec::with_capacity(len);
            for _ in 0..len {
                let offset = reader.offset();
                let read_res = E::read_i32(reader);
                values.push(attach_context(
                    "read_i32",
                    offset,
                    Some("int_array_value"),
                    read_res,
                )?);
            }
            Ok(Tag::IntArray(values))
        }
        TagType::LongArray => {
            let len = read_len_i32::<E, _>(reader, "long_array_length")?;
            let limit_offset = reader.offset();
            let limit_res =
                ensure_within_limit("long_array_length", len, config.limits.max_array_len);
            attach_context(
                "validate_size",
                limit_offset,
                Some("long_array_length"),
                limit_res,
            )?;
            let byte_len =
                checked_len_to_bytes(len, std::mem::size_of::<i64>(), "long_array_bytes")?;
            let offset = reader.offset();
            let budget_res = reader.ensure_can_read("long_array_bytes", byte_len);
            attach_context(
                "ensure_can_read",
                offset,
                Some("long_array_bytes"),
                budget_res,
            )?;
            let mut values = Vec::with_capacity(len);
            for _ in 0..len {
                let offset = reader.offset();
                let read_res = E::read_i64(reader);
                values.push(attach_context(
                    "read_i64",
                    offset,
                    Some("long_array_value"),
                    read_res,
                )?);
            }
            Ok(Tag::LongArray(values))
        }
    }
}

pub fn write_payload<E: Encoding, W: Write>(writer: &mut W, tag: &Tag) -> Result<()> {
    match tag {
        Tag::End => return Err(Error::UnexpectedEndTagPayload),
        Tag::Byte(value) => writer.write_all(&[*value as u8])?,
        Tag::Short(value) => E::write_i16(writer, *value)?,
        Tag::Int(value) => E::write_i32(writer, *value)?,
        Tag::Long(value) => E::write_i64(writer, *value)?,
        Tag::Float(value) => E::write_f32(writer, *value)?,
        Tag::Double(value) => E::write_f64(writer, *value)?,
        Tag::ByteArray(bytes) => {
            write_len_i32::<E, _>(writer, "byte_array_length", bytes.len())?;
            writer.write_all(bytes)?;
        }
        Tag::String(text) => write_string::<E, _>(writer, text)?,
        Tag::List(list) => {
            list.validate()?;
            writer.write_all(&[list.element_type.id()])?;
            E::write_list_len(writer, list.elements.len())?;
            for element in &list.elements {
                write_payload::<E, _>(writer, element)?;
            }
        }
        Tag::Compound(map) => {
            for (name, value) in map {
                if matches!(value, Tag::End) {
                    return Err(Error::UnexpectedEndTagPayload);
                }
                writer.write_all(&[value.tag_type().id()])?;
                write_string::<E, _>(writer, name)?;
                write_payload::<E, _>(writer, value)?;
            }
            writer.write_all(&[TagType::End.id()])?;
        }
        Tag::IntArray(values) => {
            write_len_i32::<E, _>(writer, "int_array_length", values.len())?;
            for value in values {
                E::write_i32(writer, *value)?;
            }
        }
        Tag::LongArray(values) => {
            write_len_i32::<E, _>(writer, "long_array_length", values.len())?;
            for value in values {
                E::write_i64(writer, *value)?;
            }
        }
    }
    Ok(())
}

fn read_tag_type<R: Read>(reader: &mut LimitedReader<R>) -> Result<TagType> {
    let mut id = [0u8; 1];
    let offset = reader.offset();
    let read_res = reader.read_exact(&mut id).map_err(Error::from);
    attach_context("read_exact", offset, Some("tag_type_id"), read_res)?;
    let tag_res = TagType::try_from(id[0]);
    attach_context("decode_tag_type", offset, Some("tag_type_id"), tag_res)
}

fn read_i8<R: Read>(reader: &mut LimitedReader<R>) -> Result<i8> {
    let mut byte = [0u8; 1];
    let offset = reader.offset();
    let read_res = reader.read_exact(&mut byte).map_err(Error::from);
    attach_context("read_exact", offset, Some("i8_value"), read_res)?;
    Ok(byte[0] as i8)
}

fn read_string<E: Encoding, R: Read>(
    reader: &mut LimitedReader<R>,
    limits: &NbtLimits,
) -> Result<String> {
    let offset = reader.offset();
    let len_res = E::read_string_len(reader);
    let len = attach_context("read_string_len", offset, Some("string_length"), len_res)?;
    let limit_offset = reader.offset();
    let limit_res = ensure_within_limit("string_length", len, limits.max_string_len);
    attach_context(
        "validate_size",
        limit_offset,
        Some("string_length"),
        limit_res,
    )?;
    let budget_offset = reader.offset();
    let budget_res = reader.ensure_can_read("string_bytes", len);
    attach_context(
        "ensure_can_read",
        budget_offset,
        Some("string_bytes"),
        budget_res,
    )?;
    let mut bytes = vec![0u8; len];
    let payload_offset = reader.offset();
    let read_res = reader.read_exact(&mut bytes).map_err(Error::from);
    attach_context("read_exact", payload_offset, Some("string_bytes"), read_res)?;
    let decode_res = String::from_utf8(bytes).map_err(|_| Error::InvalidUtf8 {
        field: "string_payload",
    });
    attach_context(
        "decode_utf8",
        payload_offset,
        Some("string_payload"),
        decode_res,
    )
}

fn write_string<E: Encoding, W: Write>(writer: &mut W, value: &str) -> Result<()> {
    E::write_string_len(writer, value.len())?;
    writer.write_all(value.as_bytes())?;
    Ok(())
}

fn read_len_i32<E: Encoding, R: Read>(
    reader: &mut LimitedReader<R>,
    field: &'static str,
) -> Result<usize> {
    let offset = reader.offset();
    let len_res = E::read_i32(reader);
    let len = attach_context("read_i32", offset, Some(field), len_res)?;
    if len < 0 {
        return Err(Error::NegativeLength { field, value: len }.with_context(
            "validate_non_negative_length",
            offset,
            Some(field),
        ));
    }
    usize::try_from(len).map_err(|_| Error::LengthOverflow {
        field,
        max: usize::MAX,
        actual: len as usize,
    })
}

fn write_len_i32<E: Encoding, W: Write>(
    writer: &mut W,
    field: &'static str,
    len: usize,
) -> Result<()> {
    if len > i32::MAX as usize {
        return Err(Error::LengthOverflow {
            field,
            max: i32::MAX as usize,
            actual: len,
        });
    }
    E::write_i32(writer, len as i32)
}

fn ensure_within_limit(field: &'static str, actual: usize, max: usize) -> Result<()> {
    if actual > max {
        return Err(Error::SizeExceeded { field, max, actual });
    }
    Ok(())
}

fn checked_len_to_bytes(count: usize, elem_size: usize, field: &'static str) -> Result<usize> {
    count.checked_mul(elem_size).ok_or(Error::LengthOverflow {
        field,
        max: usize::MAX,
        actual: count,
    })
}

struct LimitedReader<R> {
    inner: R,
    remaining: usize,
    consumed: usize,
}

impl<R> LimitedReader<R> {
    fn new(inner: R, max_read_bytes: usize) -> Self {
        Self {
            inner,
            remaining: max_read_bytes,
            consumed: 0,
        }
    }

    fn offset(&self) -> usize {
        self.consumed
    }

    fn ensure_can_read(&self, field: &'static str, size: usize) -> Result<()> {
        if size > self.remaining {
            return Err(Error::SizeExceeded {
                field,
                max: self.remaining,
                actual: size,
            });
        }
        Ok(())
    }
}

impl<R: Read> Read for LimitedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.remaining == 0 {
            return Ok(0);
        }

        let capped_len = buf.len().min(self.remaining);
        let read_len = self.inner.read(&mut buf[..capped_len])?;
        self.remaining -= read_len;
        self.consumed += read_len;
        Ok(read_len)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use indexmap::IndexMap;

    use crate::config::NbtReadConfig;
    use crate::encoding::{BigEndian, Encoding, LittleEndian, NetworkLittleEndian};
    use crate::limits::NbtLimits;

    use super::*;

    fn sample_compound_tag() -> Tag {
        let mut root = IndexMap::new();
        root.insert("health".to_string(), Tag::Int(20));
        root.insert("name".to_string(), Tag::String("Steve".to_string()));
        root.insert(
            "pos".to_string(),
            Tag::List(
                ListTag::new(
                    TagType::Float,
                    vec![Tag::Float(1.0), Tag::Float(64.0), Tag::Float(-3.5)],
                )
                .unwrap(),
            ),
        );
        root.insert("flags".to_string(), Tag::ByteArray(vec![1, 0, 1]));
        root.insert("scores".to_string(), Tag::IntArray(vec![1, 2, 3]));
        root.insert("history".to_string(), Tag::LongArray(vec![9, -3, 27]));
        Tag::Compound(root)
    }

    fn assert_array_roundtrip<E: Encoding>(tag: &Tag, tag_type: TagType) {
        let mut out = Vec::new();
        write_payload::<E, _>(&mut out, tag).unwrap();
        let mut input = Cursor::new(out);
        let decoded = read_payload::<E, _>(&mut input, tag_type).unwrap();
        assert_eq!(decoded, *tag);
        assert_eq!(decoded.tag_type(), tag_type);
    }

    #[test]
    fn be_roundtrip_compound_payload() {
        let tag = sample_compound_tag();
        let mut out = Vec::new();
        write_payload::<BigEndian, _>(&mut out, &tag).unwrap();
        let mut input = Cursor::new(out);
        let decoded = read_payload::<BigEndian, _>(&mut input, TagType::Compound).unwrap();
        assert_eq!(decoded, tag);
    }

    #[test]
    fn int_array_roundtrip_all_encodings_preserves_variant() {
        let tag = Tag::IntArray(vec![-2, -1, 0, 1, 2, i32::MIN, i32::MAX]);
        assert_array_roundtrip::<BigEndian>(&tag, TagType::IntArray);
        assert_array_roundtrip::<LittleEndian>(&tag, TagType::IntArray);
        assert_array_roundtrip::<NetworkLittleEndian>(&tag, TagType::IntArray);
    }

    #[test]
    fn long_array_roundtrip_all_encodings_preserves_variant() {
        let tag = Tag::LongArray(vec![-2, -1, 0, 1, 2, i64::MIN, i64::MAX]);
        assert_array_roundtrip::<BigEndian>(&tag, TagType::LongArray);
        assert_array_roundtrip::<LittleEndian>(&tag, TagType::LongArray);
        assert_array_roundtrip::<NetworkLittleEndian>(&tag, TagType::LongArray);
    }

    #[test]
    fn le_roundtrip_compound_payload() {
        let tag = sample_compound_tag();
        let mut out = Vec::new();
        write_payload::<LittleEndian, _>(&mut out, &tag).unwrap();
        let mut input = Cursor::new(out);
        let decoded = read_payload::<LittleEndian, _>(&mut input, TagType::Compound).unwrap();
        assert_eq!(decoded, tag);
    }

    #[test]
    fn nle_roundtrip_compound_payload() {
        let tag = sample_compound_tag();
        let mut out = Vec::new();
        write_payload::<NetworkLittleEndian, _>(&mut out, &tag).unwrap();
        let mut input = Cursor::new(out);
        let decoded =
            read_payload::<NetworkLittleEndian, _>(&mut input, TagType::Compound).unwrap();
        assert_eq!(decoded, tag);
    }

    #[test]
    fn list_constructor_rejects_mixed_types() {
        let err = ListTag::new(TagType::Int, vec![Tag::Int(1), Tag::String("bad".into())]);
        assert!(matches!(err, Err(Error::UnexpectedType { .. })));
    }

    #[test]
    fn list_decode_rejects_end_type_with_non_zero_length() {
        let payload = vec![0x00, 0x00, 0x00, 0x00, 0x01];
        let mut input = Cursor::new(payload);
        let err = read_payload::<BigEndian, _>(&mut input, TagType::List);
        let err = err.unwrap_err();
        assert!(matches!(err.innermost(), Error::InvalidListHeader { .. }));
    }

    #[test]
    fn list_decode_compatible_mode_accepts_end_type_with_non_zero_length() {
        let payload = vec![0x00, 0x00, 0x00, 0x00, 0x01];
        let mut input = Cursor::new(payload);
        let config = NbtReadConfig::compatible(NbtLimits::default());
        let decoded =
            read_payload_with_config::<BigEndian, _>(&mut input, TagType::List, &config).unwrap();
        assert_eq!(decoded, Tag::List(ListTag::empty(TagType::End)));
    }

    #[test]
    fn empty_list_encode_be_writes_elem_type_and_zero_len() {
        let int_empty = Tag::List(ListTag::empty(TagType::Int));
        let end_empty = Tag::List(ListTag::empty(TagType::End));

        let mut be_int = Vec::new();
        write_payload::<BigEndian, _>(&mut be_int, &int_empty).unwrap();
        assert_eq!(be_int, vec![TagType::Int.id(), 0x00, 0x00, 0x00, 0x00]);

        let mut be_end = Vec::new();
        write_payload::<BigEndian, _>(&mut be_end, &end_empty).unwrap();
        assert_eq!(be_end, vec![TagType::End.id(), 0x00, 0x00, 0x00, 0x00]);

        let mut input = Cursor::new(be_int);
        let decoded = read_payload::<BigEndian, _>(&mut input, TagType::List).unwrap();
        assert_eq!(decoded, int_empty);
    }

    #[test]
    fn empty_list_encode_le_writes_elem_type_and_zero_len() {
        let int_empty = Tag::List(ListTag::empty(TagType::Int));
        let end_empty = Tag::List(ListTag::empty(TagType::End));

        let mut le_int = Vec::new();
        write_payload::<LittleEndian, _>(&mut le_int, &int_empty).unwrap();
        assert_eq!(le_int, vec![TagType::Int.id(), 0x00, 0x00, 0x00, 0x00]);

        let mut le_end = Vec::new();
        write_payload::<LittleEndian, _>(&mut le_end, &end_empty).unwrap();
        assert_eq!(le_end, vec![TagType::End.id(), 0x00, 0x00, 0x00, 0x00]);

        let mut input = Cursor::new(le_int);
        let decoded = read_payload::<LittleEndian, _>(&mut input, TagType::List).unwrap();
        assert_eq!(decoded, int_empty);
    }

    #[test]
    fn empty_list_encode_nle_writes_elem_type_and_zero_len() {
        let int_empty = Tag::List(ListTag::empty(TagType::Int));
        let end_empty = Tag::List(ListTag::empty(TagType::End));

        let mut nle_int = Vec::new();
        write_payload::<NetworkLittleEndian, _>(&mut nle_int, &int_empty).unwrap();
        assert_eq!(nle_int, vec![TagType::Int.id(), 0x00]);

        let mut nle_end = Vec::new();
        write_payload::<NetworkLittleEndian, _>(&mut nle_end, &end_empty).unwrap();
        assert_eq!(nle_end, vec![TagType::End.id(), 0x00]);

        let mut input = Cursor::new(nle_int);
        let decoded = read_payload::<NetworkLittleEndian, _>(&mut input, TagType::List).unwrap();
        assert_eq!(decoded, int_empty);
    }

    #[test]
    fn byte_array_negative_length_is_rejected() {
        let payload = (-1i32).to_le_bytes().to_vec();
        let mut input = Cursor::new(payload);
        let err = read_payload::<LittleEndian, _>(&mut input, TagType::ByteArray);
        let err = err.unwrap_err();
        assert!(matches!(err.innermost(), Error::NegativeLength { .. }));
    }

    #[test]
    fn compound_rejects_unknown_inner_tag_id() {
        let payload = vec![
            99, // unknown type id
            0x00, 0x00, // empty name
            0,    // end
        ];
        let mut input = Cursor::new(payload);
        let err = read_payload::<BigEndian, _>(&mut input, TagType::Compound);
        let err = err.unwrap_err();
        assert!(matches!(err.innermost(), Error::UnknownTag { id: 99 }));
    }

    #[test]
    fn string_limit_rejects_large_string() {
        let payload = vec![0x00, 0x05, b'h', b'e', b'l', b'l', b'o'];
        let mut input = Cursor::new(payload);
        let limits = NbtLimits::default().with_max_string_len(4);
        let err = read_payload_with_limits::<BigEndian, _>(&mut input, TagType::String, &limits);
        let err = err.unwrap_err();
        assert!(matches!(
            err.innermost(),
            Error::SizeExceeded {
                field: "string_length",
                ..
            }
        ));
    }

    #[test]
    fn array_limit_rejects_large_byte_array() {
        let payload = (5i32).to_be_bytes().to_vec();
        let mut input = Cursor::new(payload);
        let limits = NbtLimits::default().with_max_array_len(4);
        let err = read_payload_with_limits::<BigEndian, _>(&mut input, TagType::ByteArray, &limits);
        let err = err.unwrap_err();
        assert!(matches!(
            err.innermost(),
            Error::SizeExceeded {
                field: "byte_array_length",
                ..
            }
        ));
    }

    #[test]
    fn read_budget_rejects_over_budget_payload() {
        let payload = vec![0x00, 0x04, b't', b'e', b's', b't'];
        let mut input = Cursor::new(payload);
        let limits = NbtLimits::default().with_max_read_bytes(3);
        let err = read_payload_with_limits::<BigEndian, _>(&mut input, TagType::String, &limits);
        let err = err.unwrap_err();
        assert!(matches!(
            err.innermost(),
            Error::SizeExceeded {
                field: "string_bytes",
                ..
            }
        ));
    }

    #[test]
    fn checked_len_to_bytes_overflow_is_rejected() {
        let err = checked_len_to_bytes(usize::MAX, 2, "int_array_bytes").unwrap_err();
        assert!(matches!(
            err,
            Error::LengthOverflow {
                field: "int_array_bytes",
                ..
            }
        ));
    }

    #[test]
    fn int_array_budget_guard_rejects_before_value_reads() {
        let payload = (4i32).to_be_bytes().to_vec();
        let mut input = Cursor::new(payload);
        let limits = NbtLimits::default().with_max_read_bytes(6);
        let err = read_payload_with_limits::<BigEndian, _>(&mut input, TagType::IntArray, &limits)
            .unwrap_err();
        assert!(matches!(
            err.innermost(),
            Error::SizeExceeded {
                field: "int_array_bytes",
                ..
            }
        ));
    }

    #[test]
    fn long_array_budget_guard_rejects_before_value_reads() {
        let payload = (4i32).to_be_bytes().to_vec();
        let mut input = Cursor::new(payload);
        let limits = NbtLimits::default().with_max_read_bytes(6);
        let err = read_payload_with_limits::<BigEndian, _>(&mut input, TagType::LongArray, &limits)
            .unwrap_err();
        assert!(matches!(
            err.innermost(),
            Error::SizeExceeded {
                field: "long_array_bytes",
                ..
            }
        ));
    }

    #[test]
    fn depth_limit_rejects_nested_compound() {
        let mut inner = IndexMap::new();
        inner.insert("value".to_string(), Tag::Int(1));

        let mut outer = IndexMap::new();
        outer.insert("nested".to_string(), Tag::Compound(inner));

        let tag = Tag::Compound(outer);
        let mut bytes = Vec::new();
        write_payload::<BigEndian, _>(&mut bytes, &tag).unwrap();

        let mut input = Cursor::new(bytes);
        let limits = NbtLimits::default().with_max_depth(1);
        let err = read_payload_with_limits::<BigEndian, _>(&mut input, TagType::Compound, &limits);
        let err = err.unwrap_err();
        assert!(matches!(err.innermost(), Error::DepthExceeded { .. }));
    }

    #[test]
    fn contextual_error_contains_op_offset_and_field() {
        let payload = vec![0x00, 0x00, 0x00, 0x00, 0x01];
        let mut input = Cursor::new(payload);
        let err = read_payload::<BigEndian, _>(&mut input, TagType::List).unwrap_err();

        assert!(err.has_context("validate_list_header", Some("list_length")));
        assert!(err.has_context("read_payload_with_config", Some("payload")));
    }

    #[test]
    fn nested_compound_end_only_closes_nested_scope() {
        let payload = vec![
            0x0A, // nested compound
            0x00, 0x06, b'n', b'e', b's', b't', b'e', b'd', // name = "nested"
            0x03, // int
            0x00, 0x01, b'a', // name = "a"
            0x00, 0x00, 0x00, 0x01, // a = 1
            0x00, // end nested compound
            0x03, // int
            0x00, 0x01, b'b', // name = "b"
            0x00, 0x00, 0x00, 0x02, // b = 2
            0x00, // end root compound
        ];

        let mut input = Cursor::new(payload);
        let decoded = read_payload::<BigEndian, _>(&mut input, TagType::Compound).unwrap();

        let mut nested = IndexMap::new();
        nested.insert("a".to_string(), Tag::Int(1));

        let mut expected = IndexMap::new();
        expected.insert("nested".to_string(), Tag::Compound(nested));
        expected.insert("b".to_string(), Tag::Int(2));

        assert_eq!(decoded, Tag::Compound(expected));
    }

    #[test]
    fn missing_compound_end_reports_contextual_io_error() {
        let payload = vec![
            0x03, // int
            0x00, 0x01, b'a', // name = "a"
            0x00, 0x00, 0x00, 0x01, // value
                  // missing TAG_End for compound
        ];
        let mut input = Cursor::new(payload);
        let err = read_payload::<BigEndian, _>(&mut input, TagType::Compound).unwrap_err();

        assert!(matches!(err.innermost(), Error::Io(_)));
        assert!(err.has_context("read_exact", Some("tag_type_id")));
    }

    #[test]
    fn write_payload_rejects_end_tag_value() {
        let mut out = Vec::new();
        let err = write_payload::<BigEndian, _>(&mut out, &Tag::End).unwrap_err();
        assert!(matches!(err, Error::UnexpectedEndTagPayload));
        assert!(out.is_empty());
    }

    #[test]
    fn compound_write_rejects_end_tag_member_without_partial_write() {
        let mut map = IndexMap::new();
        map.insert("bad".to_string(), Tag::End);

        let mut out = Vec::new();
        let err = write_payload::<BigEndian, _>(&mut out, &Tag::Compound(map)).unwrap_err();
        assert!(matches!(err, Error::UnexpectedEndTagPayload));
        assert!(out.is_empty());
    }
}
