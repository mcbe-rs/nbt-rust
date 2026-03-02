use std::io::Cursor;

use nbt_rust::{
    classify_experiment_key, read_experiments_from_root, read_tag, read_with_header_mode,
    validate_mcstructure_root, write_experiments_to_root, write_tag, write_with_header_mode,
    CompoundTag, ExperimentKeyKind, HeaderReadMode, HeaderWriteMode, ListTag, LittleEndian,
    ParseMode, ProtocolNbtAdapter, RootTag, Tag, TagType,
};

fn push_u16_le(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u32_le(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_i32_le(out: &mut Vec<u8>, value: i32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_string_le(out: &mut Vec<u8>, value: &str) {
    let bytes = value.as_bytes();
    assert!(bytes.len() <= u16::MAX as usize, "fixture string too long");
    push_u16_le(out, bytes.len() as u16);
    out.extend_from_slice(bytes);
}

fn push_named_tag_header(out: &mut Vec<u8>, tag_type: TagType, name: &str) {
    out.push(tag_type.id());
    push_string_le(out, name);
}

fn push_byte_field(out: &mut Vec<u8>, name: &str, value: i8) {
    push_named_tag_header(out, TagType::Byte, name);
    out.push(value as u8);
}

fn push_int_field(out: &mut Vec<u8>, name: &str, value: i32) {
    push_named_tag_header(out, TagType::Int, name);
    push_i32_le(out, value);
}

fn push_string_field(out: &mut Vec<u8>, name: &str, value: &str) {
    push_named_tag_header(out, TagType::String, name);
    push_string_le(out, value);
}

fn push_list_of_ints_field(out: &mut Vec<u8>, name: &str, values: &[i32]) {
    push_named_tag_header(out, TagType::List, name);
    push_list_of_ints_payload(out, values);
}

fn push_list_of_ints_payload(out: &mut Vec<u8>, values: &[i32]) {
    assert!(values.len() <= i32::MAX as usize, "fixture list too long");
    out.push(TagType::Int.id());
    push_i32_le(out, values.len() as i32);
    for value in values {
        push_i32_le(out, *value);
    }
}

fn push_end(out: &mut Vec<u8>) {
    out.push(TagType::End.id());
}

fn push_block_palette_entry_payload(out: &mut Vec<u8>, block_name: &str, version: i32) {
    push_string_field(out, "name", block_name);
    push_named_tag_header(out, TagType::Compound, "states");
    push_end(out);
    push_int_field(out, "version", version);
    push_end(out);
}

fn build_level_dat_fixture(storage_version: u32) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.push(TagType::Compound.id());
    push_string_le(&mut payload, "Data");

    push_int_field(&mut payload, "SpawnX", 128);
    push_int_field(&mut payload, "SpawnY", 64);
    push_int_field(&mut payload, "SpawnZ", -32);
    push_string_field(&mut payload, "LevelName", "FixtureWorld");
    push_byte_field(&mut payload, "experiments_ever_used", 1);
    push_byte_field(&mut payload, "saved_with_toggled_experiments", 0);

    push_named_tag_header(&mut payload, TagType::Compound, "experiments");
    push_byte_field(&mut payload, "caves_and_cliffs", 1);
    push_byte_field(&mut payload, "upcoming_creator_features", 0);
    push_byte_field(&mut payload, "test_unknown_toggle", 1);
    push_end(&mut payload);

    push_end(&mut payload);

    let mut out = Vec::new();
    push_u32_le(&mut out, storage_version);
    push_u32_le(&mut out, payload.len() as u32);
    out.extend_from_slice(&payload);
    out
}

fn build_mcstructure_fixture() -> Vec<u8> {
    let mut out = Vec::new();
    out.push(TagType::Compound.id());
    push_string_le(&mut out, "");

    push_int_field(&mut out, "format_version", 1);
    push_list_of_ints_field(&mut out, "size", &[2, 1, 2]);

    push_named_tag_header(&mut out, TagType::Compound, "structure");

    push_named_tag_header(&mut out, TagType::List, "block_indices");
    out.push(TagType::List.id());
    push_i32_le(&mut out, 2);
    push_list_of_ints_payload(&mut out, &[0, 1, -1, 0]);
    push_list_of_ints_payload(&mut out, &[-1, -1, -1, -1]);

    push_named_tag_header(&mut out, TagType::List, "entities");
    out.push(TagType::Compound.id());
    push_i32_le(&mut out, 0);

    push_named_tag_header(&mut out, TagType::List, "block_entities");
    out.push(TagType::Compound.id());
    push_i32_le(&mut out, 0);

    push_named_tag_header(&mut out, TagType::Compound, "palette");
    push_named_tag_header(&mut out, TagType::Compound, "default");

    push_named_tag_header(&mut out, TagType::List, "block_palette");
    out.push(TagType::Compound.id());
    push_i32_le(&mut out, 2);
    push_block_palette_entry_payload(&mut out, "minecraft:stone", 17959425);
    push_block_palette_entry_payload(&mut out, "minecraft:air", 17959425);

    push_named_tag_header(&mut out, TagType::Compound, "block_position_data");
    push_end(&mut out);
    push_end(&mut out);

    push_string_field(&mut out, "extra_metadata", "fixture");
    push_end(&mut out);

    push_end(&mut out);
    push_list_of_ints_field(&mut out, "structure_world_origin", &[0, 64, 0]);
    push_end(&mut out);
    out
}

fn as_compound(tag: &Tag) -> &CompoundTag {
    match tag {
        Tag::Compound(value) => value,
        other => panic!("expected compound, got {other:?}"),
    }
}

fn as_list(tag: &Tag) -> &ListTag {
    match tag {
        Tag::List(value) => value,
        other => panic!("expected list, got {other:?}"),
    }
}

fn as_int(tag: &Tag) -> i32 {
    match tag {
        Tag::Int(value) => *value,
        other => panic!("expected int, got {other:?}"),
    }
}

fn as_byte(tag: &Tag) -> i8 {
    match tag {
        Tag::Byte(value) => *value,
        other => panic!("expected byte, got {other:?}"),
    }
}

fn as_string(tag: &Tag) -> &str {
    match tag {
        Tag::String(value) => value.as_str(),
        other => panic!("expected string, got {other:?}"),
    }
}

fn root_compound(root: &RootTag) -> &CompoundTag {
    as_compound(&root.payload)
}

fn as_compound_mut(tag: &mut Tag) -> &mut CompoundTag {
    match tag {
        Tag::Compound(value) => value,
        other => panic!("expected compound, got {other:?}"),
    }
}

fn as_list_mut(tag: &mut Tag) -> &mut ListTag {
    match tag {
        Tag::List(value) => value,
        other => panic!("expected list, got {other:?}"),
    }
}

#[test]
fn level_dat_fixture_header_decode_encode_roundtrip() {
    let storage_version = 10u32;
    let bytes = build_level_dat_fixture(storage_version);

    let mut cursor = Cursor::new(bytes.as_slice());
    let root =
        read_with_header_mode::<LittleEndian, _>(&mut cursor, HeaderReadMode::LevelDatHeader)
            .unwrap();
    assert_eq!(cursor.position() as usize, bytes.len());
    assert_eq!(root.name, "Data");

    let root = root_compound(&root);
    assert_eq!(as_int(root.get("SpawnX").unwrap()), 128);
    assert_eq!(as_int(root.get("SpawnY").unwrap()), 64);
    assert_eq!(as_int(root.get("SpawnZ").unwrap()), -32);
    assert_eq!(as_string(root.get("LevelName").unwrap()), "FixtureWorld");
    assert_eq!(as_byte(root.get("experiments_ever_used").unwrap()), 1);

    let experiments = as_compound(root.get("experiments").unwrap());
    assert_eq!(as_byte(experiments.get("caves_and_cliffs").unwrap()), 1);
    assert_eq!(
        as_byte(experiments.get("upcoming_creator_features").unwrap()),
        0
    );
    assert_eq!(as_byte(experiments.get("test_unknown_toggle").unwrap()), 1);

    let mut reencoded = Vec::new();
    write_with_header_mode::<LittleEndian, _>(
        &mut reencoded,
        &RootTag::new("Data", Tag::Compound(root.clone())),
        HeaderWriteMode::LevelDatHeader { storage_version },
    )
    .unwrap();
    assert_eq!(reencoded, bytes);
}

#[test]
fn level_dat_experiments_unknown_keys_preserved_with_registry() {
    let storage_version = 11u32;
    let bytes = build_level_dat_fixture(storage_version);
    let mut cursor = Cursor::new(bytes);
    let mut root =
        read_with_header_mode::<LittleEndian, _>(&mut cursor, HeaderReadMode::LevelDatHeader)
            .unwrap();

    let mut experiments = read_experiments_from_root(&root).unwrap();
    assert_eq!(experiments.get("caves_and_cliffs"), Some(1));
    assert_eq!(experiments.get("test_unknown_toggle"), Some(1));
    assert_eq!(
        classify_experiment_key("caves_and_cliffs"),
        ExperimentKeyKind::Known
    );
    assert_eq!(
        classify_experiment_key("test_unknown_toggle"),
        ExperimentKeyKind::Unknown
    );

    experiments.set("new_future_toggle", 1);
    write_experiments_to_root(&mut root, &experiments).unwrap();

    let mut reencoded = Vec::new();
    write_with_header_mode::<LittleEndian, _>(
        &mut reencoded,
        &root,
        HeaderWriteMode::LevelDatHeader { storage_version },
    )
    .unwrap();

    let mut cursor = Cursor::new(reencoded);
    let root =
        read_with_header_mode::<LittleEndian, _>(&mut cursor, HeaderReadMode::LevelDatHeader)
            .unwrap();
    let roundtrip = read_experiments_from_root(&root).unwrap();

    assert_eq!(roundtrip.get("test_unknown_toggle"), Some(1));
    assert_eq!(roundtrip.get("new_future_toggle"), Some(1));
}

#[test]
fn mcstructure_fixture_decode_shape_and_roundtrip() {
    let bytes = build_mcstructure_fixture();

    let mut cursor = Cursor::new(bytes.as_slice());
    let root = read_tag::<LittleEndian, _>(&mut cursor).unwrap();
    assert_eq!(cursor.position() as usize, bytes.len());
    assert_eq!(root.name, "");

    let top = root_compound(&root);
    assert_eq!(as_int(top.get("format_version").unwrap()), 1);

    let size = as_list(top.get("size").unwrap());
    assert_eq!(size.element_type, TagType::Int);
    assert_eq!(
        size.elements,
        vec![Tag::Int(2), Tag::Int(1), Tag::Int(2)],
        "size list should match fixture dimensions"
    );

    let structure = as_compound(top.get("structure").unwrap());
    let block_indices = as_list(structure.get("block_indices").unwrap());
    assert_eq!(block_indices.element_type, TagType::List);
    assert_eq!(block_indices.elements.len(), 2);

    let primary_layer = as_list(&block_indices.elements[0]);
    let secondary_layer = as_list(&block_indices.elements[1]);
    assert_eq!(primary_layer.element_type, TagType::Int);
    assert_eq!(secondary_layer.element_type, TagType::Int);
    assert_eq!(primary_layer.elements.len(), 4);
    assert_eq!(secondary_layer.elements.len(), 4);

    let origin = as_list(top.get("structure_world_origin").unwrap());
    assert_eq!(origin.element_type, TagType::Int);
    assert_eq!(
        origin.elements,
        vec![Tag::Int(0), Tag::Int(64), Tag::Int(0)]
    );

    let mut reencoded = Vec::new();
    write_tag::<LittleEndian, _>(&mut reencoded, &root).unwrap();
    assert_eq!(reencoded, bytes);
}

#[test]
fn mcstructure_fixture_strict_semantic_validation_passes() {
    let bytes = build_mcstructure_fixture();
    let root = read_tag::<LittleEndian, _>(&mut Cursor::new(bytes)).unwrap();
    let report = validate_mcstructure_root(&root, ParseMode::Strict).unwrap();
    assert_eq!(report.size, [2, 1, 2]);
    assert_eq!(report.volume, 4);
    assert_eq!(report.layer_count, 2);
    assert_eq!(report.palette_len, 2);
}

#[test]
fn mcstructure_fixture_strict_rejects_layer_count_mismatch() {
    let bytes = build_mcstructure_fixture();
    let mut root = read_tag::<LittleEndian, _>(&mut Cursor::new(bytes)).unwrap();

    let top = as_compound_mut(&mut root.payload);
    let structure = as_compound_mut(top.get_mut("structure").unwrap());
    let block_indices = as_list_mut(structure.get_mut("block_indices").unwrap());
    block_indices.elements.pop();

    let err = validate_mcstructure_root(&root, ParseMode::Strict).unwrap_err();
    assert!(matches!(
        err,
        nbt_rust::Error::InvalidStructureShape {
            detail: "mcstructure_block_indices_layer_count_must_be_two"
        }
    ));
}

#[test]
fn mcstructure_fixture_compatible_accepts_out_of_range_indices() {
    let bytes = build_mcstructure_fixture();
    let mut root = read_tag::<LittleEndian, _>(&mut Cursor::new(bytes)).unwrap();

    let top = as_compound_mut(&mut root.payload);
    let structure = as_compound_mut(top.get_mut("structure").unwrap());
    let block_indices = as_list_mut(structure.get_mut("block_indices").unwrap());
    let primary = as_list_mut(block_indices.elements.get_mut(0).unwrap());
    primary.elements[0] = Tag::Int(999);
    primary.elements[1] = Tag::Int(-2);

    let report = validate_mcstructure_root(&root, ParseMode::Compatible).unwrap();
    assert_eq!(report.out_of_range_indices, 2);
}

#[test]
fn protocol_adapter_decodes_fixture_payload_roots() {
    let adapter = ProtocolNbtAdapter::little_endian();

    let level_dat = build_level_dat_fixture(11);
    let level_dat_payload = &level_dat[8..];
    let level_root = adapter.decode_root_tag(level_dat_payload).unwrap();
    assert_eq!(level_root.name, "Data");
    assert!(root_compound(&level_root).contains_key("experiments"));

    let mcstructure = build_mcstructure_fixture();
    let mcstructure_root = adapter.decode_root_tag(&mcstructure).unwrap();
    let top = root_compound(&mcstructure_root);
    assert!(top.contains_key("format_version"));
    assert!(top.contains_key("structure"));
    assert!(top.contains_key("structure_world_origin"));
}
