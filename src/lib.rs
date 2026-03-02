//! nbt-rust core library.

pub mod codec_bridge;
pub mod config;
pub mod core;
pub mod encoding;
pub mod error;
pub mod experiments;
pub mod headless;
pub mod limits;
pub mod mcstructure;
pub mod protocol_adapter;
pub mod root;
pub mod serde_api;
pub mod tag;

pub use codec_bridge::{NbtCodecFacade, NbtCodecProfile};
pub use config::{NbtReadConfig, ParseMode};
pub use core::{read_payload, read_payload_with_config, read_payload_with_limits, write_payload};
pub use encoding::{
    BigEndian, BigEndianCodec, Codec, Encoding, EncodingKind, LittleEndian, LittleEndianCodec,
    NetworkLittleEndian, NetworkLittleEndianCodec, BE, LE, NLE,
};
pub use error::{Error, Result};
pub use experiments::{
    classify_experiment_key, is_known_experiment_key, read_experiments_from_root,
    write_experiments_to_root, ExperimentKeyKind, Experiments, KNOWN_EXPERIMENT_KEYS,
};
pub use headless::{
    from_headless_bytes, read_headless, read_headless_by_id, read_headless_by_id_with_config,
    read_headless_by_id_with_limits, read_headless_prefixed, read_headless_prefixed_with_config,
    read_headless_prefixed_with_limits, read_headless_with_config, read_headless_with_limits,
    read_value, read_value_with_config, read_value_with_limits, to_headless_bytes, write_headless,
    write_headless_prefixed, write_value,
};
pub use limits::NbtLimits;
pub use mcstructure::{
    validate_mcstructure_root, validate_mcstructure_tag, zyx_flatten_index, zyx_unflatten_index,
    McStructureSemanticReport,
};
pub use protocol_adapter::{ProtocolNbtAdapter, ProtocolNbtEncoding};
pub use root::{
    read_tag, read_tag_with_config, read_tag_with_limits, read_with_header_mode,
    read_with_header_mode_with_config, read_with_header_mode_with_limits, write_tag,
    write_with_header_mode, HeaderReadMode, HeaderWriteMode, RootTag, BEDROCK_FILE_HEADER_MAGIC,
};
pub use serde_api::{
    from_be_bytes, from_be_bytes_with_config, from_byte_array_tag, from_le_bytes,
    from_le_bytes_with_config, from_net_bytes, from_net_bytes_with_config, from_root_tag, from_tag,
    to_be_bytes, to_be_bytes_named, to_byte_array_tag, to_le_bytes, to_le_bytes_named,
    to_net_bytes, to_net_bytes_named, to_root_tag, to_tag, NbtByteArray, SerdeBehaviorContract,
    SERDE_BEHAVIOR_CONTRACT,
};
pub use tag::{CompoundTag, ListTag, Tag, TagType};
