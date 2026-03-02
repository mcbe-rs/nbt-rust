use crate::config::ParseMode;
use crate::error::{Error, Result};
use crate::root::RootTag;
use crate::tag::{CompoundTag, ListTag, Tag, TagType};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct McStructureSemanticReport {
    pub size: [usize; 3],
    pub volume: usize,
    pub layer_count: usize,
    pub palette_len: usize,
    pub has_default_palette: bool,
    pub no_block_indices: usize,
    pub out_of_range_indices: usize,
    pub invalid_block_position_data_keys: usize,
}

pub fn validate_mcstructure_root(
    root: &RootTag,
    parse_mode: ParseMode,
) -> Result<McStructureSemanticReport> {
    validate_mcstructure_tag(&root.payload, parse_mode)
}

pub fn validate_mcstructure_tag(
    payload: &Tag,
    parse_mode: ParseMode,
) -> Result<McStructureSemanticReport> {
    let top = expect_compound(payload, "mcstructure_root_payload_type")?;
    let _format_version = validate_format_version(top, parse_mode)?;
    let size = parse_size(top)?;
    validate_origin(top)?;
    let volume = checked_volume(size)?;

    let structure = expect_compound(
        required(top, "structure", "mcstructure_structure_missing")?,
        "mcstructure_structure_type",
    )?;

    let block_indices = expect_list(
        required(
            structure,
            "block_indices",
            "mcstructure_block_indices_missing",
        )?,
        "mcstructure_block_indices_type",
    )?;
    if block_indices.element_type != TagType::List {
        return Err(Error::InvalidStructureShape {
            detail: "mcstructure_block_indices_not_list_of_list",
        });
    }
    if block_indices.elements.len() != 2 {
        return Err(Error::InvalidStructureShape {
            detail: "mcstructure_block_indices_layer_count_must_be_two",
        });
    }

    let (palette_len, has_default_palette, block_position_data) =
        resolve_palette_semantics(structure, parse_mode)?;

    let mut report = McStructureSemanticReport {
        size,
        volume,
        layer_count: block_indices.elements.len(),
        palette_len,
        has_default_palette,
        ..McStructureSemanticReport::default()
    };

    for layer_tag in &block_indices.elements {
        let layer = expect_list(layer_tag, "mcstructure_block_indices_layer_type")?;
        if layer.element_type != TagType::Int && parse_mode == ParseMode::Strict {
            return Err(Error::InvalidStructureShape {
                detail: "mcstructure_block_indices_layer_not_int_list",
            });
        }
        if layer.elements.len() != volume {
            return Err(Error::InvalidStructureShape {
                detail: "mcstructure_block_indices_length_mismatch",
            });
        }
        validate_layer_indices(layer, palette_len, parse_mode, &mut report)?;
    }

    if let Some(position_data) = block_position_data {
        validate_block_position_data_keys(position_data, volume, parse_mode, &mut report)?;
    }

    Ok(report)
}

pub fn zyx_flatten_index(size: [usize; 3], x: usize, y: usize, z: usize) -> Result<usize> {
    if x >= size[0] || y >= size[1] || z >= size[2] {
        return Err(Error::InvalidStructureShape {
            detail: "mcstructure_coordinate_out_of_bounds",
        });
    }
    let base = x
        .checked_mul(size[1])
        .and_then(|v| v.checked_add(y))
        .ok_or(Error::LengthOverflow {
            field: "mcstructure_flatten_index",
            max: usize::MAX,
            actual: usize::MAX,
        })?;
    let flat = base.checked_mul(size[2]).ok_or(Error::LengthOverflow {
        field: "mcstructure_flatten_index",
        max: usize::MAX,
        actual: usize::MAX,
    })?;
    flat.checked_add(z).ok_or(Error::LengthOverflow {
        field: "mcstructure_flatten_index",
        max: usize::MAX,
        actual: usize::MAX,
    })
}

pub fn zyx_unflatten_index(size: [usize; 3], flat_index: usize) -> Result<(usize, usize, usize)> {
    let volume = checked_volume(size)?;
    if flat_index >= volume {
        return Err(Error::InvalidStructureShape {
            detail: "mcstructure_flat_index_out_of_bounds",
        });
    }
    let yz_span = size[1] * size[2];
    let x = flat_index / yz_span;
    let rem = flat_index % yz_span;
    let y = rem / size[2];
    let z = rem % size[2];
    Ok((x, y, z))
}

fn required<'a>(
    compound: &'a CompoundTag,
    key: &'static str,
    detail: &'static str,
) -> Result<&'a Tag> {
    compound
        .get(key)
        .ok_or(Error::InvalidStructureShape { detail })
}

fn expect_compound<'a>(tag: &'a Tag, context: &'static str) -> Result<&'a CompoundTag> {
    match tag {
        Tag::Compound(value) => Ok(value),
        other => Err(Error::UnexpectedType {
            context,
            expected_id: TagType::Compound.id(),
            actual_id: other.tag_type().id(),
        }),
    }
}

fn expect_list<'a>(tag: &'a Tag, context: &'static str) -> Result<&'a ListTag> {
    match tag {
        Tag::List(value) => Ok(value),
        other => Err(Error::UnexpectedType {
            context,
            expected_id: TagType::List.id(),
            actual_id: other.tag_type().id(),
        }),
    }
}

fn expect_int(tag: &Tag, context: &'static str) -> Result<i32> {
    match tag {
        Tag::Int(value) => Ok(*value),
        other => Err(Error::UnexpectedType {
            context,
            expected_id: TagType::Int.id(),
            actual_id: other.tag_type().id(),
        }),
    }
}

fn parse_size(top: &CompoundTag) -> Result<[usize; 3]> {
    let size = expect_list(
        required(top, "size", "mcstructure_size_missing")?,
        "mcstructure_size_type",
    )?;
    if size.element_type != TagType::Int || size.elements.len() != 3 {
        return Err(Error::InvalidStructureShape {
            detail: "mcstructure_size_must_be_int3",
        });
    }

    let mut out = [0usize; 3];
    for (index, value_tag) in size.elements.iter().enumerate() {
        let value = expect_int(value_tag, "mcstructure_size_value_type")?;
        if value < 0 {
            return Err(Error::InvalidStructureShape {
                detail: "mcstructure_size_negative_component",
            });
        }
        out[index] = value as usize;
    }
    Ok(out)
}

fn validate_format_version(top: &CompoundTag, parse_mode: ParseMode) -> Result<i32> {
    let format_version = required(top, "format_version", "mcstructure_format_version_missing")?;
    let value = expect_int(format_version, "mcstructure_format_version_type")?;
    if parse_mode == ParseMode::Strict && value != 1 {
        return Err(Error::InvalidStructureShape {
            detail: "mcstructure_format_version_must_be_one",
        });
    }
    Ok(value)
}

fn validate_origin(top: &CompoundTag) -> Result<()> {
    let origin = expect_list(
        required(
            top,
            "structure_world_origin",
            "mcstructure_world_origin_missing",
        )?,
        "mcstructure_world_origin_type",
    )?;
    if origin.element_type != TagType::Int || origin.elements.len() != 3 {
        return Err(Error::InvalidStructureShape {
            detail: "mcstructure_world_origin_must_be_int3",
        });
    }
    for value in &origin.elements {
        let _ = expect_int(value, "mcstructure_world_origin_value_type")?;
    }
    Ok(())
}

fn checked_volume(size: [usize; 3]) -> Result<usize> {
    size[0]
        .checked_mul(size[1])
        .and_then(|value| value.checked_mul(size[2]))
        .ok_or(Error::LengthOverflow {
            field: "mcstructure_volume",
            max: usize::MAX,
            actual: usize::MAX,
        })
}

fn resolve_palette_semantics(
    structure: &CompoundTag,
    parse_mode: ParseMode,
) -> Result<(usize, bool, Option<&CompoundTag>)> {
    let palette = expect_compound(
        required(structure, "palette", "mcstructure_palette_missing")?,
        "mcstructure_palette_type",
    )?;
    let Some(default_tag) = palette.get("default") else {
        if parse_mode == ParseMode::Strict {
            return Err(Error::InvalidStructureShape {
                detail: "mcstructure_default_palette_missing",
            });
        }
        return Ok((0, false, None));
    };

    let default = expect_compound(default_tag, "mcstructure_default_palette_type")?;
    let block_palette = expect_list(
        required(
            default,
            "block_palette",
            "mcstructure_block_palette_missing",
        )?,
        "mcstructure_block_palette_type",
    )?;
    if block_palette.element_type != TagType::Compound {
        return Err(Error::InvalidStructureShape {
            detail: "mcstructure_block_palette_not_compound_list",
        });
    }

    let block_position_data = match default.get("block_position_data") {
        Some(tag) => Some(expect_compound(
            tag,
            "mcstructure_block_position_data_type",
        )?),
        None => None,
    };

    Ok((block_palette.elements.len(), true, block_position_data))
}

fn validate_layer_indices(
    layer: &ListTag,
    palette_len: usize,
    parse_mode: ParseMode,
    report: &mut McStructureSemanticReport,
) -> Result<()> {
    for index_tag in &layer.elements {
        let index = match index_tag {
            Tag::Int(value) => *value,
            _ if parse_mode == ParseMode::Compatible => 0,
            _ => {
                return Err(Error::UnexpectedType {
                    context: "mcstructure_block_index_value_type",
                    expected_id: TagType::Int.id(),
                    actual_id: index_tag.tag_type().id(),
                })
            }
        };
        if index == -1 {
            report.no_block_indices += 1;
            continue;
        }
        if index < -1 || (index as usize) >= palette_len {
            if parse_mode == ParseMode::Strict {
                return Err(Error::InvalidPaletteIndex { index, palette_len });
            }
            report.out_of_range_indices += 1;
        }
    }
    Ok(())
}

fn validate_block_position_data_keys(
    block_position_data: &CompoundTag,
    volume: usize,
    parse_mode: ParseMode,
    report: &mut McStructureSemanticReport,
) -> Result<()> {
    for key in block_position_data.keys() {
        let flat = match key.parse::<usize>() {
            Ok(value) => value,
            Err(_) => {
                if parse_mode == ParseMode::Strict {
                    return Err(Error::InvalidStructureShape {
                        detail: "mcstructure_block_position_data_key_not_usize",
                    });
                }
                report.invalid_block_position_data_keys += 1;
                continue;
            }
        };
        if flat >= volume {
            if parse_mode == ParseMode::Strict {
                return Err(Error::InvalidStructureShape {
                    detail: "mcstructure_block_position_data_key_out_of_bounds",
                });
            }
            report.invalid_block_position_data_keys += 1;
            continue;
        }

        // Contract check: declared ZYX flatten/unflatten mapping must stay stable.
        let (x, y, z) = zyx_unflatten_index(report.size, flat)?;
        let roundtrip = zyx_flatten_index(report.size, x, y, z)?;
        if roundtrip != flat {
            return Err(Error::InvalidStructureShape {
                detail: "mcstructure_zyx_roundtrip_mismatch",
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;

    use super::*;

    fn build_valid_mcstructure_root() -> RootTag {
        let mut root = IndexMap::new();
        root.insert("format_version".to_string(), Tag::Int(1));
        root.insert(
            "size".to_string(),
            Tag::List(
                ListTag::new(TagType::Int, vec![Tag::Int(2), Tag::Int(1), Tag::Int(2)]).unwrap(),
            ),
        );

        let mut structure = IndexMap::new();
        let primary = Tag::List(
            ListTag::new(
                TagType::Int,
                vec![Tag::Int(0), Tag::Int(1), Tag::Int(-1), Tag::Int(0)],
            )
            .unwrap(),
        );
        let secondary = Tag::List(
            ListTag::new(
                TagType::Int,
                vec![Tag::Int(-1), Tag::Int(-1), Tag::Int(-1), Tag::Int(-1)],
            )
            .unwrap(),
        );
        structure.insert(
            "block_indices".to_string(),
            Tag::List(ListTag::new(TagType::List, vec![primary, secondary]).unwrap()),
        );

        let mut default = IndexMap::new();
        default.insert(
            "block_palette".to_string(),
            Tag::List(
                ListTag::new(
                    TagType::Compound,
                    vec![
                        Tag::Compound(IndexMap::new()),
                        Tag::Compound(IndexMap::new()),
                    ],
                )
                .unwrap(),
            ),
        );
        let mut palette = IndexMap::new();
        palette.insert("default".to_string(), Tag::Compound(default));
        structure.insert("palette".to_string(), Tag::Compound(palette));

        root.insert("structure".to_string(), Tag::Compound(structure));
        root.insert(
            "structure_world_origin".to_string(),
            Tag::List(
                ListTag::new(TagType::Int, vec![Tag::Int(0), Tag::Int(64), Tag::Int(0)]).unwrap(),
            ),
        );
        RootTag::new("", Tag::Compound(root))
    }

    #[test]
    fn strict_validator_accepts_valid_fixture_shape() {
        let root = build_valid_mcstructure_root();
        let report = validate_mcstructure_root(&root, ParseMode::Strict).unwrap();
        assert_eq!(report.size, [2, 1, 2]);
        assert_eq!(report.volume, 4);
        assert_eq!(report.layer_count, 2);
        assert_eq!(report.palette_len, 2);
        assert_eq!(report.no_block_indices, 5);
        assert_eq!(report.out_of_range_indices, 0);
    }

    #[test]
    fn strict_validator_requires_format_version() {
        let mut root = build_valid_mcstructure_root();
        let top = match &mut root.payload {
            Tag::Compound(value) => value,
            _ => unreachable!(),
        };
        top.shift_remove("format_version");

        let err = validate_mcstructure_root(&root, ParseMode::Strict).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidStructureShape {
                detail: "mcstructure_format_version_missing"
            }
        ));
    }

    #[test]
    fn strict_validator_rejects_non_one_format_version() {
        let mut root = build_valid_mcstructure_root();
        let top = match &mut root.payload {
            Tag::Compound(value) => value,
            _ => unreachable!(),
        };
        top.insert("format_version".to_string(), Tag::Int(2));

        let err = validate_mcstructure_root(&root, ParseMode::Strict).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidStructureShape {
                detail: "mcstructure_format_version_must_be_one"
            }
        ));
    }

    #[test]
    fn compatible_validator_accepts_non_one_format_version() {
        let mut root = build_valid_mcstructure_root();
        let top = match &mut root.payload {
            Tag::Compound(value) => value,
            _ => unreachable!(),
        };
        top.insert("format_version".to_string(), Tag::Int(2));

        let report = validate_mcstructure_root(&root, ParseMode::Compatible).unwrap();
        assert_eq!(report.volume, 4);
    }

    #[test]
    fn strict_validator_rejects_missing_default_palette() {
        let mut root = build_valid_mcstructure_root();
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
        palette.shift_remove("default");

        let err = validate_mcstructure_root(&root, ParseMode::Strict).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidStructureShape {
                detail: "mcstructure_default_palette_missing"
            }
        ));
    }

    #[test]
    fn compatible_validator_accepts_missing_default_palette() {
        let mut root = build_valid_mcstructure_root();
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
        palette.shift_remove("default");

        let report = validate_mcstructure_root(&root, ParseMode::Compatible).unwrap();
        assert!(!report.has_default_palette);
        assert_eq!(report.palette_len, 0);
    }

    #[test]
    fn strict_validator_rejects_out_of_range_palette_index() {
        let mut root = build_valid_mcstructure_root();
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
        primary.elements[0] = Tag::Int(99);

        let err = validate_mcstructure_root(&root, ParseMode::Strict).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidPaletteIndex {
                index: 99,
                palette_len: 2
            }
        ));
    }

    #[test]
    fn compatible_validator_falls_back_for_non_int_layer_entries() {
        let mut root = build_valid_mcstructure_root();
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

        let non_int_layer = Tag::List(
            ListTag::new(
                TagType::Byte,
                vec![Tag::Byte(5), Tag::Byte(6), Tag::Byte(7), Tag::Byte(8)],
            )
            .unwrap(),
        );
        layers.elements[0] = non_int_layer;

        let report = validate_mcstructure_root(&root, ParseMode::Compatible).unwrap();
        assert_eq!(report.out_of_range_indices, 0);
    }

    #[test]
    fn compatible_validator_counts_out_of_range_palette_indices() {
        let mut root = build_valid_mcstructure_root();
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
        primary.elements[0] = Tag::Int(-2);
        primary.elements[1] = Tag::Int(9);

        let report = validate_mcstructure_root(&root, ParseMode::Compatible).unwrap();
        assert_eq!(report.out_of_range_indices, 2);
    }

    #[test]
    fn zyx_flatten_and_unflatten_roundtrip() {
        let size = [2, 3, 4];
        for x in 0..size[0] {
            for y in 0..size[1] {
                for z in 0..size[2] {
                    let flat = zyx_flatten_index(size, x, y, z).unwrap();
                    let (rx, ry, rz) = zyx_unflatten_index(size, flat).unwrap();
                    assert_eq!((rx, ry, rz), (x, y, z));
                }
            }
        }
    }
}
