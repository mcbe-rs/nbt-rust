use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::config::NbtReadConfig;
use crate::error::Result;
use crate::protocol_adapter::{ProtocolNbtAdapter, ProtocolNbtEncoding};
use crate::tag::TagType;

/// Codec profile used by the protocol bridge facade.
pub trait NbtCodecProfile {
    const NBT_ENCODING: ProtocolNbtEncoding;

    fn nbt_read_config() -> NbtReadConfig {
        NbtReadConfig::default()
    }
}

/// High-level facade for protocol packet structs.
///
/// This is the macro/facade layer for "annotation-like" usage:
/// `nbt_profile!(MyPacket, net);` then use `encode_nbt_*`/`decode_nbt_*`.
pub trait NbtCodecFacade: Serialize + DeserializeOwned + NbtCodecProfile + Sized {
    fn nbt_adapter() -> ProtocolNbtAdapter {
        ProtocolNbtAdapter {
            encoding: Self::NBT_ENCODING,
            read_config: Self::nbt_read_config(),
        }
    }

    fn encode_nbt_root(&self, root_name: impl Into<String>) -> Result<Vec<u8>> {
        Self::nbt_adapter().encode_root(root_name, self)
    }

    fn decode_nbt_root(bytes: &[u8]) -> Result<Self> {
        Self::nbt_adapter().decode_root(bytes)
    }

    fn decode_nbt_root_named(bytes: &[u8]) -> Result<(String, Self)> {
        Self::nbt_adapter().decode_root_named(bytes)
    }

    fn encode_nbt_prefixed(&self) -> Result<Vec<u8>> {
        Self::nbt_adapter().encode_prefixed(self)
    }

    fn decode_nbt_prefixed(bytes: &[u8]) -> Result<Self> {
        Self::nbt_adapter().decode_prefixed(bytes)
    }

    fn encode_nbt_headless(&self) -> Result<(TagType, Vec<u8>)> {
        Self::nbt_adapter().encode_headless(self)
    }

    fn decode_nbt_headless(tag_type: TagType, bytes: &[u8]) -> Result<Self> {
        Self::nbt_adapter().decode_headless(tag_type, bytes)
    }
}

impl<T> NbtCodecFacade for T where T: Serialize + DeserializeOwned + NbtCodecProfile {}

/// Declare a codec profile for a type.
///
/// `net` => NetworkLittleEndian, `le` => LittleEndian, `be` => BigEndian.
#[macro_export]
macro_rules! nbt_profile {
    ($ty:ty, net) => {
        impl $crate::codec_bridge::NbtCodecProfile for $ty {
            const NBT_ENCODING: $crate::ProtocolNbtEncoding = $crate::ProtocolNbtEncoding::Network;
        }
    };
    ($ty:ty, le) => {
        impl $crate::codec_bridge::NbtCodecProfile for $ty {
            const NBT_ENCODING: $crate::ProtocolNbtEncoding =
                $crate::ProtocolNbtEncoding::LittleEndian;
        }
    };
    ($ty:ty, be) => {
        impl $crate::codec_bridge::NbtCodecProfile for $ty {
            const NBT_ENCODING: $crate::ProtocolNbtEncoding =
                $crate::ProtocolNbtEncoding::BigEndian;
        }
    };
}

/// Declare a codec profile with explicit read config.
#[macro_export]
macro_rules! nbt_profile_with_config {
    ($ty:ty, net, $config:expr) => {
        impl $crate::codec_bridge::NbtCodecProfile for $ty {
            const NBT_ENCODING: $crate::ProtocolNbtEncoding = $crate::ProtocolNbtEncoding::Network;
            fn nbt_read_config() -> $crate::NbtReadConfig {
                $config
            }
        }
    };
    ($ty:ty, le, $config:expr) => {
        impl $crate::codec_bridge::NbtCodecProfile for $ty {
            const NBT_ENCODING: $crate::ProtocolNbtEncoding =
                $crate::ProtocolNbtEncoding::LittleEndian;
            fn nbt_read_config() -> $crate::NbtReadConfig {
                $config
            }
        }
    };
    ($ty:ty, be, $config:expr) => {
        impl $crate::codec_bridge::NbtCodecProfile for $ty {
            const NBT_ENCODING: $crate::ProtocolNbtEncoding =
                $crate::ProtocolNbtEncoding::BigEndian;
            fn nbt_read_config() -> $crate::NbtReadConfig {
                $config
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use crate::config::NbtReadConfig;
    use crate::limits::NbtLimits;
    use crate::{Error, TagType};

    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct NetPacket {
        username: String,
        hp: i32,
        flags: Vec<u8>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct LePacket {
        id: i32,
        values: Vec<i32>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct StrictListPacket {
        entries: Vec<i32>,
    }

    crate::nbt_profile!(NetPacket, net);
    crate::nbt_profile!(LePacket, le);
    crate::nbt_profile_with_config!(
        StrictListPacket,
        be,
        NbtReadConfig::strict(NbtLimits::default())
    );

    #[test]
    fn net_profile_root_roundtrip() {
        let input = NetPacket {
            username: "Steve".to_string(),
            hp: 20,
            flags: vec![1, 0, 1],
        };
        let bytes = input.encode_nbt_root("Packet").unwrap();
        let (root_name, output) = NetPacket::decode_nbt_root_named(&bytes).unwrap();
        assert_eq!(root_name, "Packet");
        assert_eq!(output, input);
    }

    #[test]
    fn le_profile_prefixed_roundtrip() {
        let input = LePacket {
            id: 7,
            values: vec![2, 4, 6, 8],
        };
        let bytes = input.encode_nbt_prefixed().unwrap();
        let output = LePacket::decode_nbt_prefixed(&bytes).unwrap();
        assert_eq!(output, input);
    }

    #[test]
    fn headless_profile_roundtrip() {
        let input = LePacket {
            id: 99,
            values: vec![1, 3, 5],
        };
        let (tag_type, bytes) = input.encode_nbt_headless().unwrap();
        let output = LePacket::decode_nbt_headless(tag_type, &bytes).unwrap();
        assert_eq!(output, input);
    }

    #[test]
    fn profile_with_config_uses_declared_parse_mode() {
        // BE list payload: elem_type=TAG_End, len=1 (invalid in strict mode)
        let bytes = vec![0x00, 0x00, 0x00, 0x00, 0x01];
        let err = StrictListPacket::decode_nbt_headless(TagType::List, &bytes).unwrap_err();
        assert!(matches!(err.innermost(), Error::InvalidListHeader { .. }));
    }
}
