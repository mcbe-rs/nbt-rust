use std::io::Cursor;

use nbt_rust::{
    read_payload_with_config, read_tag_with_config, validate_mcstructure_root, write_tag,
    BigEndian, CompoundTag, Error, ListTag, NbtLimits, NbtReadConfig, NetworkLittleEndian,
    ParseMode, RootTag, Tag, TagType,
};

fn int_list(values: &[i32]) -> Tag {
    Tag::List(ListTag::new(TagType::Int, values.iter().copied().map(Tag::Int).collect()).unwrap())
}

fn nested_list(depth: usize) -> Tag {
    if depth == 0 {
        Tag::List(ListTag::empty(TagType::End))
    } else {
        Tag::List(ListTag::new(TagType::List, vec![nested_list(depth - 1)]).unwrap())
    }
}

fn base_mcstructure_root() -> RootTag {
    let mut top = CompoundTag::new();
    top.insert("format_version".to_string(), Tag::Int(1));
    top.insert("size".to_string(), int_list(&[2, 1, 2]));

    let primary = int_list(&[0, 1, -1, 0]);
    let secondary = int_list(&[-1, -1, -1, -1]);
    let mut structure = CompoundTag::new();
    structure.insert(
        "block_indices".to_string(),
        Tag::List(ListTag::new(TagType::List, vec![primary, secondary]).unwrap()),
    );

    let mut default_palette = CompoundTag::new();
    default_palette.insert(
        "block_palette".to_string(),
        Tag::List(
            ListTag::new(
                TagType::Compound,
                vec![
                    Tag::Compound(CompoundTag::new()),
                    Tag::Compound(CompoundTag::new()),
                ],
            )
            .unwrap(),
        ),
    );
    default_palette.insert(
        "block_position_data".to_string(),
        Tag::Compound(CompoundTag::new()),
    );

    let mut palette = CompoundTag::new();
    palette.insert("default".to_string(), Tag::Compound(default_palette));
    structure.insert("palette".to_string(), Tag::Compound(palette));

    top.insert("structure".to_string(), Tag::Compound(structure));
    top.insert("structure_world_origin".to_string(), int_list(&[0, 64, 0]));
    RootTag::new("", Tag::Compound(top))
}

#[test]
fn depth_bomb_fixture_is_rejected_by_configured_depth_limit() {
    let root = RootTag::new("depth", nested_list(20));
    let mut bytes = Vec::new();
    write_tag::<BigEndian, _>(&mut bytes, &root).unwrap();

    let cfg = NbtReadConfig::strict(NbtLimits::default().with_max_depth(8));
    let err = read_tag_with_config::<BigEndian, _>(&mut Cursor::new(bytes), &cfg).unwrap_err();
    assert!(matches!(
        err.innermost(),
        Error::DepthExceeded { max_depth: 8, .. }
    ));
}

#[test]
fn pathological_byte_array_length_is_rejected_before_allocation() {
    let mut payload = Vec::new();
    payload.extend_from_slice(&(5000i32).to_be_bytes());

    let cfg = NbtReadConfig::strict(NbtLimits::default().with_max_array_len(1024));
    let err = read_payload_with_config::<BigEndian, _>(
        &mut Cursor::new(payload),
        TagType::ByteArray,
        &cfg,
    )
    .unwrap_err();
    assert!(matches!(
        err.innermost(),
        Error::SizeExceeded {
            field: "byte_array_length",
            max: 1024,
            actual: 5000
        }
    ));
}

#[test]
fn truncated_network_varint_fixture_is_rejected() {
    let cfg = NbtReadConfig::default();
    let err = read_payload_with_config::<NetworkLittleEndian, _>(
        &mut Cursor::new(vec![0x80]),
        TagType::String,
        &cfg,
    )
    .unwrap_err();
    assert!(matches!(
        err.innermost(),
        Error::InvalidVarint {
            detail: "truncated u32 varint"
        }
    ));
}

#[test]
fn mcstructure_pathological_layer_length_mismatch_is_rejected() {
    let mut root = base_mcstructure_root();
    let top = match &mut root.payload {
        Tag::Compound(value) => value,
        _ => unreachable!(),
    };
    let structure = match top.get_mut("structure").unwrap() {
        Tag::Compound(value) => value,
        _ => unreachable!(),
    };
    let layers = match structure.get_mut("block_indices").unwrap() {
        Tag::List(value) => value,
        _ => unreachable!(),
    };
    let primary = match layers.elements.get_mut(0).unwrap() {
        Tag::List(value) => value,
        _ => unreachable!(),
    };
    primary.elements.pop();

    let err = validate_mcstructure_root(&root, ParseMode::Strict).unwrap_err();
    assert!(matches!(
        err,
        Error::InvalidStructureShape {
            detail: "mcstructure_block_indices_length_mismatch"
        }
    ));
}

#[test]
fn mcstructure_pathological_block_position_key_behavior_strict_vs_compatible() {
    let mut root = base_mcstructure_root();
    let top = match &mut root.payload {
        Tag::Compound(value) => value,
        _ => unreachable!(),
    };
    let structure = match top.get_mut("structure").unwrap() {
        Tag::Compound(value) => value,
        _ => unreachable!(),
    };
    let palette = match structure.get_mut("palette").unwrap() {
        Tag::Compound(value) => value,
        _ => unreachable!(),
    };
    let default = match palette.get_mut("default").unwrap() {
        Tag::Compound(value) => value,
        _ => unreachable!(),
    };
    let position_data = match default.get_mut("block_position_data").unwrap() {
        Tag::Compound(value) => value,
        _ => unreachable!(),
    };
    position_data.insert(
        "not_a_flat_index".to_string(),
        Tag::Compound(CompoundTag::new()),
    );

    let strict_err = validate_mcstructure_root(&root, ParseMode::Strict).unwrap_err();
    assert!(matches!(
        strict_err,
        Error::InvalidStructureShape {
            detail: "mcstructure_block_position_data_key_not_usize"
        }
    ));

    let report = validate_mcstructure_root(&root, ParseMode::Compatible).unwrap();
    assert_eq!(report.invalid_block_position_data_keys, 1);
}
