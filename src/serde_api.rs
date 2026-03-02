use std::collections::BTreeMap;
use std::io::Cursor;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_value::Value as SerdeValue;

use crate::config::NbtReadConfig;
use crate::encoding::{BigEndian, LittleEndian, NetworkLittleEndian};
use crate::error::{Error, Result};
use crate::root::{read_tag_with_config, write_tag, RootTag};
use crate::tag::{CompoundTag, ListTag, Tag, TagType};

/// Typed serde conversion contract for `Option` and byte vectors.
///
/// - `Option::Some(T)` is serialized as `T`'s NBT tag payload.
/// - `Option::None` is rejected (NBT has no native null marker).
/// - Non-empty `Vec<u8>` is detected and encoded as `Tag::ByteArray`.
/// - Empty `Vec<u8>` is ambiguous under `serde_value` (type erasure); to force
///   byte-array semantics for empty payloads, use [`NbtByteArray`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SerdeBehaviorContract {
    pub option_some_as_inner_tag: bool,
    pub option_none_rejected: bool,
    pub vec_u8_non_empty_as_byte_array: bool,
    pub empty_vec_u8_requires_wrapper: bool,
}

pub const SERDE_BEHAVIOR_CONTRACT: SerdeBehaviorContract = SerdeBehaviorContract {
    option_some_as_inner_tag: true,
    option_none_rejected: true,
    vec_u8_non_empty_as_byte_array: true,
    empty_vec_u8_requires_wrapper: true,
};

/// Wrapper that guarantees `Tag::ByteArray` semantics (including empty payloads)
/// when used with typed serde conversion APIs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NbtByteArray(pub Vec<u8>);

impl Serialize for NbtByteArray {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for NbtByteArray {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self(Vec::<u8>::deserialize(deserializer)?))
    }
}

pub fn to_byte_array_tag(bytes: impl Into<Vec<u8>>) -> Tag {
    Tag::ByteArray(bytes.into())
}

pub fn from_byte_array_tag(tag: &Tag) -> Result<Vec<u8>> {
    match tag {
        Tag::ByteArray(value) => Ok(value.clone()),
        other => Err(Error::UnexpectedType {
            context: "byte_array_tag_decode",
            expected_id: TagType::ByteArray.id(),
            actual_id: other.tag_type().id(),
        }),
    }
}

pub fn to_tag<T: Serialize>(value: &T) -> Result<Tag> {
    let raw = serde_value::to_value(value).map_err(serde_error)?;
    serde_value_to_tag(raw)
}

pub fn from_tag<T: DeserializeOwned>(tag: &Tag) -> Result<T> {
    let raw = tag_to_serde_value(tag)?;
    raw.deserialize_into().map_err(serde_error)
}

pub fn to_root_tag<T: Serialize>(name: impl Into<String>, value: &T) -> Result<RootTag> {
    Ok(RootTag::new(name, to_tag(value)?))
}

pub fn from_root_tag<T: DeserializeOwned>(root: &RootTag) -> Result<T> {
    from_tag(&root.payload)
}

pub fn to_be_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    to_be_bytes_named("", value)
}

pub fn to_le_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    to_le_bytes_named("", value)
}

pub fn to_net_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    to_net_bytes_named("", value)
}

pub fn to_be_bytes_named<T: Serialize>(name: impl Into<String>, value: &T) -> Result<Vec<u8>> {
    let root = to_root_tag(name, value)?;
    let mut out = Vec::new();
    write_tag::<BigEndian, _>(&mut out, &root)?;
    Ok(out)
}

pub fn to_le_bytes_named<T: Serialize>(name: impl Into<String>, value: &T) -> Result<Vec<u8>> {
    let root = to_root_tag(name, value)?;
    let mut out = Vec::new();
    write_tag::<LittleEndian, _>(&mut out, &root)?;
    Ok(out)
}

pub fn to_net_bytes_named<T: Serialize>(name: impl Into<String>, value: &T) -> Result<Vec<u8>> {
    let root = to_root_tag(name, value)?;
    let mut out = Vec::new();
    write_tag::<NetworkLittleEndian, _>(&mut out, &root)?;
    Ok(out)
}

pub fn from_be_bytes<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    from_be_bytes_with_config(bytes, &NbtReadConfig::default())
}

pub fn from_le_bytes<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    from_le_bytes_with_config(bytes, &NbtReadConfig::default())
}

pub fn from_net_bytes<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    from_net_bytes_with_config(bytes, &NbtReadConfig::default())
}

pub fn from_be_bytes_with_config<T: DeserializeOwned>(
    bytes: &[u8],
    config: &NbtReadConfig,
) -> Result<T> {
    let mut cursor = Cursor::new(bytes);
    let root = read_tag_with_config::<BigEndian, _>(&mut cursor, config)?;
    ensure_consumed(bytes.len(), cursor.position() as usize)?;
    from_root_tag(&root)
}

pub fn from_le_bytes_with_config<T: DeserializeOwned>(
    bytes: &[u8],
    config: &NbtReadConfig,
) -> Result<T> {
    let mut cursor = Cursor::new(bytes);
    let root = read_tag_with_config::<LittleEndian, _>(&mut cursor, config)?;
    ensure_consumed(bytes.len(), cursor.position() as usize)?;
    from_root_tag(&root)
}

pub fn from_net_bytes_with_config<T: DeserializeOwned>(
    bytes: &[u8],
    config: &NbtReadConfig,
) -> Result<T> {
    let mut cursor = Cursor::new(bytes);
    let root = read_tag_with_config::<NetworkLittleEndian, _>(&mut cursor, config)?;
    ensure_consumed(bytes.len(), cursor.position() as usize)?;
    from_root_tag(&root)
}

fn ensure_consumed(total: usize, consumed: usize) -> Result<()> {
    if consumed == total {
        return Ok(());
    }
    Err(Error::TrailingPayloadBytes {
        unread: total - consumed,
    })
}

fn serde_error<E: std::fmt::Display>(error: E) -> Error {
    Error::Serde {
        message: error.to_string(),
    }
}

fn serde_value_to_tag(value: SerdeValue) -> Result<Tag> {
    match value {
        SerdeValue::Bool(value) => Ok(Tag::Byte(if value { 1 } else { 0 })),
        SerdeValue::I8(value) => Ok(Tag::Byte(value)),
        SerdeValue::I16(value) => Ok(Tag::Short(value)),
        SerdeValue::I32(value) => Ok(Tag::Int(value)),
        SerdeValue::I64(value) => Ok(Tag::Long(value)),
        SerdeValue::U8(value) => Ok(Tag::Short(value as i16)),
        SerdeValue::U16(value) => i16::try_from(value)
            .map(Tag::Short)
            .or_else(|_| Ok(Tag::Int(i32::from(value)))),
        SerdeValue::U32(value) => {
            if let Ok(int) = i32::try_from(value) {
                Ok(Tag::Int(int))
            } else {
                Ok(Tag::Long(i64::from(value)))
            }
        }
        SerdeValue::U64(value) => {
            let long = i64::try_from(value).map_err(|_| serde_error("u64 out of i64 range"))?;
            Ok(Tag::Long(long))
        }
        SerdeValue::F32(value) => Ok(Tag::Float(value)),
        SerdeValue::F64(value) => Ok(Tag::Double(value)),
        SerdeValue::Char(value) => Ok(Tag::String(value.to_string())),
        SerdeValue::String(value) => Ok(Tag::String(value)),
        SerdeValue::Bytes(bytes) => Ok(Tag::ByteArray(bytes)),
        SerdeValue::Seq(values) => serde_seq_to_tag(values),
        SerdeValue::Map(values) => serde_map_to_compound(values).map(Tag::Compound),
        SerdeValue::Option(None) => Err(serde_error("Option::None is not representable in NBT")),
        SerdeValue::Option(Some(inner)) => serde_value_to_tag(*inner),
        SerdeValue::Unit => Err(serde_error("unit values are not representable in NBT")),
        SerdeValue::Newtype(inner) => serde_value_to_tag(*inner),
    }
}

fn serde_seq_to_tag(values: Vec<SerdeValue>) -> Result<Tag> {
    if values.is_empty() {
        return Ok(Tag::List(ListTag::empty(TagType::End)));
    }

    if let Some(bytes) = try_u8_seq_to_byte_array(&values) {
        return Ok(Tag::ByteArray(bytes));
    }
    if let Some(ints) = try_i32_seq_to_int_array(&values) {
        return Ok(Tag::IntArray(ints));
    }
    if let Some(longs) = try_i64_seq_to_long_array(&values) {
        return Ok(Tag::LongArray(longs));
    }

    let mut tags = Vec::with_capacity(values.len());
    for value in values {
        tags.push(serde_value_to_tag(value)?);
    }
    let element_type = tags.first().map(Tag::tag_type).unwrap_or(TagType::End);
    Ok(Tag::List(ListTag::new(element_type, tags)?))
}

fn try_u8_seq_to_byte_array(values: &[SerdeValue]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        match value {
            SerdeValue::U8(byte) => out.push(*byte),
            _ => return None,
        }
    }
    Some(out)
}

fn try_i32_seq_to_int_array(values: &[SerdeValue]) -> Option<Vec<i32>> {
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        match value {
            SerdeValue::I32(int) => out.push(*int),
            SerdeValue::U32(int) => out.push(i32::try_from(*int).ok()?),
            _ => return None,
        }
    }
    Some(out)
}

fn try_i64_seq_to_long_array(values: &[SerdeValue]) -> Option<Vec<i64>> {
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        match value {
            SerdeValue::I64(long) => out.push(*long),
            SerdeValue::U64(long) => out.push(i64::try_from(*long).ok()?),
            _ => return None,
        }
    }
    Some(out)
}

fn serde_map_to_compound(values: BTreeMap<SerdeValue, SerdeValue>) -> Result<CompoundTag> {
    let mut out = CompoundTag::new();
    for (key, value) in values {
        let key = serde_key_to_string(key)?;
        let value = serde_value_to_tag(value)?;
        out.insert(key, value);
    }
    Ok(out)
}

fn serde_key_to_string(value: SerdeValue) -> Result<String> {
    match value {
        SerdeValue::String(value) => Ok(value),
        SerdeValue::Char(value) => Ok(value.to_string()),
        SerdeValue::Bool(value) => Ok(value.to_string()),
        SerdeValue::I8(value) => Ok(value.to_string()),
        SerdeValue::I16(value) => Ok(value.to_string()),
        SerdeValue::I32(value) => Ok(value.to_string()),
        SerdeValue::I64(value) => Ok(value.to_string()),
        SerdeValue::U8(value) => Ok(value.to_string()),
        SerdeValue::U16(value) => Ok(value.to_string()),
        SerdeValue::U32(value) => Ok(value.to_string()),
        SerdeValue::U64(value) => Ok(value.to_string()),
        _ => Err(serde_error("map key must be string-like for NBT compound")),
    }
}

fn tag_to_serde_value(tag: &Tag) -> Result<SerdeValue> {
    match tag {
        Tag::End => Err(serde_error(
            "TAG_End is not representable as a typed serde value",
        )),
        Tag::Byte(value) => Ok(SerdeValue::I8(*value)),
        Tag::Short(value) => Ok(SerdeValue::I16(*value)),
        Tag::Int(value) => Ok(SerdeValue::I32(*value)),
        Tag::Long(value) => Ok(SerdeValue::I64(*value)),
        Tag::Float(value) => Ok(SerdeValue::F32(*value)),
        Tag::Double(value) => Ok(SerdeValue::F64(*value)),
        Tag::ByteArray(values) => Ok(SerdeValue::Seq(
            values.iter().copied().map(SerdeValue::U8).collect(),
        )),
        Tag::String(value) => Ok(SerdeValue::String(value.clone())),
        Tag::List(list) => {
            let mut values = Vec::with_capacity(list.elements.len());
            for element in &list.elements {
                values.push(tag_to_serde_value(element)?);
            }
            Ok(SerdeValue::Seq(values))
        }
        Tag::Compound(values) => {
            let mut map = BTreeMap::new();
            for (key, value) in values {
                map.insert(SerdeValue::String(key.clone()), tag_to_serde_value(value)?);
            }
            Ok(SerdeValue::Map(map))
        }
        Tag::IntArray(values) => Ok(SerdeValue::Seq(
            values.iter().copied().map(SerdeValue::I32).collect(),
        )),
        Tag::LongArray(values) => Ok(SerdeValue::Seq(
            values.iter().copied().map(SerdeValue::I64).collect(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;
    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct DemoData {
        name: String,
        health: i32,
        pitch: f32,
        bytes: Vec<u8>,
        scores: Vec<i32>,
    }

    fn sample() -> DemoData {
        DemoData {
            name: "Steve".to_string(),
            health: 20,
            pitch: 11.5,
            bytes: vec![1, 2, 3, 250],
            scores: vec![7, 11, 42],
        }
    }

    #[test]
    fn tag_roundtrip_typed() {
        let input = sample();
        let tag = to_tag(&input).unwrap();
        let output: DemoData = from_tag(&tag).unwrap();
        assert_eq!(output, input);
    }

    #[test]
    fn be_bytes_roundtrip_typed() {
        let input = sample();
        let bytes = to_be_bytes(&input).unwrap();
        let output: DemoData = from_be_bytes(&bytes).unwrap();
        assert_eq!(output, input);
    }

    #[test]
    fn le_bytes_roundtrip_typed() {
        let input = sample();
        let bytes = to_le_bytes_named("demo", &input).unwrap();
        let output: DemoData = from_le_bytes(&bytes).unwrap();
        assert_eq!(output, input);
    }

    #[test]
    fn net_bytes_roundtrip_typed() {
        let input = sample();
        let bytes = to_net_bytes(&input).unwrap();
        let output: DemoData = from_net_bytes(&bytes).unwrap();
        assert_eq!(output, input);
    }

    #[test]
    fn none_option_is_rejected() {
        #[derive(Serialize)]
        struct OptionalField {
            maybe: Option<i32>,
        }

        let err = to_tag(&OptionalField { maybe: None }).unwrap_err();
        assert!(matches!(err.innermost(), Error::Serde { .. }));
    }

    #[test]
    fn some_option_is_serialized_as_inner_tag() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct OptionalField {
            maybe: Option<i32>,
        }

        let tag = to_tag(&OptionalField { maybe: Some(42) }).unwrap();
        let compound = match tag {
            Tag::Compound(value) => value,
            other => panic!("expected compound, got {other:?}"),
        };
        assert_eq!(compound.get("maybe"), Some(&Tag::Int(42)));

        let decoded: OptionalField = from_tag(&Tag::Compound(compound)).unwrap();
        assert_eq!(decoded, OptionalField { maybe: Some(42) });
    }

    #[test]
    fn vec_u8_non_empty_encodes_as_byte_array() {
        let tag = to_tag(&vec![1u8, 2, 3, 250]).unwrap();
        assert_eq!(tag, Tag::ByteArray(vec![1, 2, 3, 250]));
    }

    #[test]
    fn empty_vec_u8_without_wrapper_is_encoded_as_empty_list() {
        let tag = to_tag(&Vec::<u8>::new()).unwrap();
        assert_eq!(tag, Tag::List(ListTag::empty(TagType::End)));
        let contract = std::hint::black_box(SERDE_BEHAVIOR_CONTRACT);
        assert!(contract.empty_vec_u8_requires_wrapper);
    }

    #[test]
    fn nbt_byte_array_wrapper_forces_empty_byte_array_semantics() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct WrappedBytes {
            bytes: NbtByteArray,
        }

        let input = WrappedBytes {
            bytes: NbtByteArray(Vec::new()),
        };
        let tag = to_tag(&input).unwrap();
        let compound = match tag {
            Tag::Compound(value) => value,
            other => panic!("expected compound, got {other:?}"),
        };
        assert_eq!(compound.get("bytes"), Some(&Tag::ByteArray(Vec::new())));

        let output: WrappedBytes = from_tag(&Tag::Compound(compound)).unwrap();
        assert_eq!(output, input);
    }

    #[test]
    fn byte_array_helper_roundtrip() {
        let tag = to_byte_array_tag(vec![9u8, 8, 7]);
        let bytes = from_byte_array_tag(&tag).unwrap();
        assert_eq!(bytes, vec![9, 8, 7]);

        let wrong = Tag::Int(1);
        let err = from_byte_array_tag(&wrong).unwrap_err();
        assert!(matches!(
            err,
            Error::UnexpectedType {
                context: "byte_array_tag_decode",
                expected_id,
                actual_id
            } if expected_id == TagType::ByteArray.id() && actual_id == TagType::Int.id()
        ));
    }

    #[test]
    fn contract_flags_are_expected() {
        let contract = std::hint::black_box(SERDE_BEHAVIOR_CONTRACT);
        assert!(contract.option_some_as_inner_tag);
        assert!(contract.option_none_rejected);
        assert!(contract.vec_u8_non_empty_as_byte_array);
        assert!(contract.empty_vec_u8_requires_wrapper);
    }

    #[test]
    fn byte_array_tag_decodes_to_vec_u8() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct ByteVecHolder {
            bytes: Vec<u8>,
        }

        let mut compound = IndexMap::new();
        compound.insert("bytes".to_string(), Tag::ByteArray(vec![4, 5, 6]));
        let decoded: ByteVecHolder = from_tag(&Tag::Compound(compound)).unwrap();
        assert_eq!(
            decoded,
            ByteVecHolder {
                bytes: vec![4, 5, 6]
            }
        );
    }
}
