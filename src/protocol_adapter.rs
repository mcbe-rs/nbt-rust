use std::io::Cursor;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::config::{NbtReadConfig, ParseMode};
use crate::encoding::{BigEndian, LittleEndian, NetworkLittleEndian};
use crate::error::{Error, Result};
use crate::headless::{
    read_headless_prefixed_with_config, read_headless_with_config, write_headless,
    write_headless_prefixed,
};
use crate::limits::NbtLimits;
use crate::root::{read_tag_with_config, write_tag, RootTag};
use crate::serde_api::{from_root_tag, from_tag, to_root_tag, to_tag};
use crate::tag::{Tag, TagType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolNbtEncoding {
    Network,
    LittleEndian,
    BigEndian,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolNbtAdapter {
    pub encoding: ProtocolNbtEncoding,
    pub read_config: NbtReadConfig,
}

impl Default for ProtocolNbtAdapter {
    fn default() -> Self {
        Self::network()
    }
}

impl ProtocolNbtAdapter {
    pub fn network() -> Self {
        Self {
            encoding: ProtocolNbtEncoding::Network,
            read_config: NbtReadConfig::default(),
        }
    }

    pub fn little_endian() -> Self {
        Self {
            encoding: ProtocolNbtEncoding::LittleEndian,
            read_config: NbtReadConfig::default(),
        }
    }

    pub fn big_endian() -> Self {
        Self {
            encoding: ProtocolNbtEncoding::BigEndian,
            read_config: NbtReadConfig::default(),
        }
    }

    pub fn with_config(mut self, read_config: NbtReadConfig) -> Self {
        self.read_config = read_config;
        self
    }

    pub fn with_limits(mut self, limits: NbtLimits) -> Self {
        self.read_config = self.read_config.with_limits(limits);
        self
    }

    pub fn with_parse_mode(mut self, parse_mode: ParseMode) -> Self {
        self.read_config = self.read_config.with_parse_mode(parse_mode);
        self
    }

    pub fn decode_headless_tag(&self, tag_type: TagType, bytes: &[u8]) -> Result<Tag> {
        let mut cursor = Cursor::new(bytes);
        let tag = match self.encoding {
            ProtocolNbtEncoding::Network => read_headless_with_config::<NetworkLittleEndian, _>(
                &mut cursor,
                tag_type,
                &self.read_config,
            ),
            ProtocolNbtEncoding::LittleEndian => read_headless_with_config::<LittleEndian, _>(
                &mut cursor,
                tag_type,
                &self.read_config,
            ),
            ProtocolNbtEncoding::BigEndian => {
                read_headless_with_config::<BigEndian, _>(&mut cursor, tag_type, &self.read_config)
            }
        }?;
        ensure_fully_consumed(bytes.len(), cursor.position() as usize)?;
        Ok(tag)
    }

    pub fn decode_headless<T: DeserializeOwned>(
        &self,
        tag_type: TagType,
        bytes: &[u8],
    ) -> Result<T> {
        let tag = self.decode_headless_tag(tag_type, bytes)?;
        from_tag(&tag)
    }

    pub fn encode_headless_tag(&self, tag: &Tag) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        match self.encoding {
            ProtocolNbtEncoding::Network => {
                write_headless::<NetworkLittleEndian, _>(&mut out, tag)?
            }
            ProtocolNbtEncoding::LittleEndian => write_headless::<LittleEndian, _>(&mut out, tag)?,
            ProtocolNbtEncoding::BigEndian => write_headless::<BigEndian, _>(&mut out, tag)?,
        }
        Ok(out)
    }

    pub fn encode_headless<T: Serialize>(&self, value: &T) -> Result<(TagType, Vec<u8>)> {
        let tag = to_tag(value)?;
        let tag_type = tag.tag_type();
        let bytes = self.encode_headless_tag(&tag)?;
        Ok((tag_type, bytes))
    }

    pub fn decode_prefixed_tag(&self, bytes: &[u8]) -> Result<Tag> {
        let mut cursor = Cursor::new(bytes);
        let tag = match self.encoding {
            ProtocolNbtEncoding::Network => read_headless_prefixed_with_config::<
                NetworkLittleEndian,
                _,
            >(&mut cursor, &self.read_config),
            ProtocolNbtEncoding::LittleEndian => read_headless_prefixed_with_config::<
                LittleEndian,
                _,
            >(&mut cursor, &self.read_config),
            ProtocolNbtEncoding::BigEndian => {
                read_headless_prefixed_with_config::<BigEndian, _>(&mut cursor, &self.read_config)
            }
        }?;
        ensure_fully_consumed(bytes.len(), cursor.position() as usize)?;
        Ok(tag)
    }

    pub fn decode_prefixed<T: DeserializeOwned>(&self, bytes: &[u8]) -> Result<T> {
        let tag = self.decode_prefixed_tag(bytes)?;
        from_tag(&tag)
    }

    pub fn encode_prefixed_tag(&self, tag: &Tag) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        match self.encoding {
            ProtocolNbtEncoding::Network => {
                write_headless_prefixed::<NetworkLittleEndian, _>(&mut out, tag)?
            }
            ProtocolNbtEncoding::LittleEndian => {
                write_headless_prefixed::<LittleEndian, _>(&mut out, tag)?
            }
            ProtocolNbtEncoding::BigEndian => {
                write_headless_prefixed::<BigEndian, _>(&mut out, tag)?
            }
        }
        Ok(out)
    }

    pub fn encode_prefixed<T: Serialize>(&self, value: &T) -> Result<Vec<u8>> {
        let tag = to_tag(value)?;
        self.encode_prefixed_tag(&tag)
    }

    pub fn decode_root_tag(&self, bytes: &[u8]) -> Result<RootTag> {
        let mut cursor = Cursor::new(bytes);
        let root = match self.encoding {
            ProtocolNbtEncoding::Network => {
                read_tag_with_config::<NetworkLittleEndian, _>(&mut cursor, &self.read_config)
            }
            ProtocolNbtEncoding::LittleEndian => {
                read_tag_with_config::<LittleEndian, _>(&mut cursor, &self.read_config)
            }
            ProtocolNbtEncoding::BigEndian => {
                read_tag_with_config::<BigEndian, _>(&mut cursor, &self.read_config)
            }
        }?;
        ensure_fully_consumed(bytes.len(), cursor.position() as usize)?;
        Ok(root)
    }

    pub fn decode_root<T: DeserializeOwned>(&self, bytes: &[u8]) -> Result<T> {
        let root = self.decode_root_tag(bytes)?;
        from_root_tag(&root)
    }

    pub fn decode_root_named<T: DeserializeOwned>(&self, bytes: &[u8]) -> Result<(String, T)> {
        let root = self.decode_root_tag(bytes)?;
        let value = from_root_tag(&root)?;
        Ok((root.name, value))
    }

    pub fn encode_root_tag(&self, root: &RootTag) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        match self.encoding {
            ProtocolNbtEncoding::Network => write_tag::<NetworkLittleEndian, _>(&mut out, root)?,
            ProtocolNbtEncoding::LittleEndian => write_tag::<LittleEndian, _>(&mut out, root)?,
            ProtocolNbtEncoding::BigEndian => write_tag::<BigEndian, _>(&mut out, root)?,
        }
        Ok(out)
    }

    pub fn encode_root<T: Serialize>(
        &self,
        root_name: impl Into<String>,
        value: &T,
    ) -> Result<Vec<u8>> {
        let root = to_root_tag(root_name, value)?;
        self.encode_root_tag(&root)
    }
}

fn ensure_fully_consumed(total: usize, consumed: usize) -> Result<()> {
    if consumed == total {
        return Ok(());
    }
    Err(Error::TrailingPayloadBytes {
        unread: total - consumed,
    })
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use crate::config::NbtReadConfig;
    use crate::limits::NbtLimits;
    use crate::tag::ListTag;

    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct PlayerState {
        username: String,
        hp: i32,
        scores: Vec<i32>,
        flags: Vec<u8>,
    }

    fn sample() -> PlayerState {
        PlayerState {
            username: "Alex".to_string(),
            hp: 20,
            scores: vec![10, 20, 30],
            flags: vec![1, 0, 1, 1],
        }
    }

    #[test]
    fn network_headless_typed_roundtrip() {
        let adapter = ProtocolNbtAdapter::network();
        let input = sample();
        let (tag_type, bytes) = adapter.encode_headless(&input).unwrap();
        let output: PlayerState = adapter.decode_headless(tag_type, &bytes).unwrap();
        assert_eq!(output, input);
    }

    #[test]
    fn network_prefixed_typed_roundtrip() {
        let adapter = ProtocolNbtAdapter::network();
        let input = sample();
        let bytes = adapter.encode_prefixed(&input).unwrap();
        let output: PlayerState = adapter.decode_prefixed(&bytes).unwrap();
        assert_eq!(output, input);
    }

    #[test]
    fn little_endian_root_named_roundtrip() {
        let adapter = ProtocolNbtAdapter::little_endian();
        let input = sample();
        let bytes = adapter.encode_root("PlayerState", &input).unwrap();
        let (root_name, output): (String, PlayerState) = adapter.decode_root_named(&bytes).unwrap();
        assert_eq!(root_name, "PlayerState");
        assert_eq!(output, input);
    }

    #[test]
    fn strict_vs_compatible_list_header_behavior() {
        let payload = vec![0x00, 0x00, 0x00, 0x00, 0x01];

        let strict = ProtocolNbtAdapter::big_endian();
        let strict_err = strict
            .decode_headless_tag(TagType::List, &payload)
            .unwrap_err();
        assert!(matches!(
            strict_err.innermost(),
            Error::InvalidListHeader { .. }
        ));

        let compat_cfg = NbtReadConfig::compatible(NbtLimits::default());
        let compat = ProtocolNbtAdapter::big_endian().with_config(compat_cfg);
        let compat_tag = compat.decode_headless_tag(TagType::List, &payload).unwrap();
        assert_eq!(compat_tag, Tag::List(ListTag::empty(TagType::End)));
    }

    #[test]
    fn prefixed_decode_rejects_trailing_bytes() {
        let adapter = ProtocolNbtAdapter::network();
        let bytes = {
            let mut out = adapter.encode_prefixed(&sample()).unwrap();
            out.extend_from_slice(&[0xAA, 0xBB]);
            out
        };
        let err = adapter.decode_prefixed_tag(&bytes).unwrap_err();
        assert!(matches!(
            err.innermost(),
            Error::TrailingPayloadBytes { unread: 2 }
        ));
    }
}
