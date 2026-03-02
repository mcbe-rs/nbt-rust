use indexmap::IndexMap;

use crate::error::{Error, Result};
use crate::root::RootTag;
use crate::tag::{CompoundTag, Tag, TagType};

pub const KNOWN_EXPERIMENT_KEYS: &[&str] = &[
    "caves_and_cliffs",
    "upcoming_creator_features",
    "gametest",
    "holiday_creator_features",
    "data_driven_items",
    "custom_biomes",
    "next_major_update",
    "molang_features",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExperimentKeyKind {
    Known,
    Unknown,
}

pub fn classify_experiment_key(key: &str) -> ExperimentKeyKind {
    if is_known_experiment_key(key) {
        ExperimentKeyKind::Known
    } else {
        ExperimentKeyKind::Unknown
    }
}

pub fn is_known_experiment_key(key: &str) -> bool {
    KNOWN_EXPERIMENT_KEYS.contains(&key)
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Experiments {
    flags: IndexMap<String, i8>,
}

impl Experiments {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_compound(compound: &CompoundTag) -> Result<Self> {
        let mut flags = IndexMap::with_capacity(compound.len());
        for (key, value) in compound {
            let flag = match value {
                Tag::Byte(flag) => *flag,
                other => {
                    return Err(Error::UnexpectedType {
                        context: "level_dat_experiments_value_type",
                        expected_id: TagType::Byte.id(),
                        actual_id: other.tag_type().id(),
                    });
                }
            };
            flags.insert(key.clone(), flag);
        }
        Ok(Self { flags })
    }

    pub fn to_compound(&self) -> CompoundTag {
        let mut out = CompoundTag::with_capacity(self.flags.len());
        for (key, value) in &self.flags {
            out.insert(key.clone(), Tag::Byte(*value));
        }
        out
    }

    pub fn get(&self, key: &str) -> Option<i8> {
        self.flags.get(key).copied()
    }

    pub fn set(&mut self, key: impl Into<String>, value: i8) -> Option<i8> {
        self.flags.insert(key.into(), value)
    }

    pub fn remove(&mut self, key: &str) -> Option<i8> {
        self.flags.shift_remove(key)
    }

    pub fn len(&self) -> usize {
        self.flags.len()
    }

    pub fn is_empty(&self) -> bool {
        self.flags.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, i8)> {
        self.flags.iter().map(|(key, value)| (key.as_str(), *value))
    }

    pub fn iter_known(&self) -> impl Iterator<Item = (&str, i8)> {
        self.iter()
            .filter(|(key, _)| classify_experiment_key(key) == ExperimentKeyKind::Known)
    }

    pub fn iter_unknown(&self) -> impl Iterator<Item = (&str, i8)> {
        self.iter()
            .filter(|(key, _)| classify_experiment_key(key) == ExperimentKeyKind::Unknown)
    }
}

pub fn read_experiments_from_root(root: &RootTag) -> Result<Experiments> {
    let top = match &root.payload {
        Tag::Compound(value) => value,
        other => {
            return Err(Error::UnexpectedType {
                context: "level_dat_root_payload_type",
                expected_id: TagType::Compound.id(),
                actual_id: other.tag_type().id(),
            });
        }
    };
    let experiments_tag = top.get("experiments").ok_or(Error::InvalidStructureShape {
        detail: "level_dat_experiments_missing",
    })?;
    let experiments_compound = match experiments_tag {
        Tag::Compound(value) => value,
        other => {
            return Err(Error::UnexpectedType {
                context: "level_dat_experiments_type",
                expected_id: TagType::Compound.id(),
                actual_id: other.tag_type().id(),
            });
        }
    };
    Experiments::from_compound(experiments_compound)
}

pub fn write_experiments_to_root(root: &mut RootTag, experiments: &Experiments) -> Result<()> {
    let top = match &mut root.payload {
        Tag::Compound(value) => value,
        other => {
            return Err(Error::UnexpectedType {
                context: "level_dat_root_payload_type",
                expected_id: TagType::Compound.id(),
                actual_id: other.tag_type().id(),
            });
        }
    };
    top.insert(
        "experiments".to_string(),
        Tag::Compound(experiments.to_compound()),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_classifies_known_and_unknown_keys() {
        assert!(is_known_experiment_key("caves_and_cliffs"));
        assert_eq!(
            classify_experiment_key("caves_and_cliffs"),
            ExperimentKeyKind::Known
        );

        assert!(!is_known_experiment_key("my_future_toggle"));
        assert_eq!(
            classify_experiment_key("my_future_toggle"),
            ExperimentKeyKind::Unknown
        );
    }

    #[test]
    fn experiments_roundtrip_preserves_unknown_keys() {
        let mut map = CompoundTag::new();
        map.insert("caves_and_cliffs".to_string(), Tag::Byte(1));
        map.insert("my_future_toggle".to_string(), Tag::Byte(1));
        map.insert("another_unknown".to_string(), Tag::Byte(0));

        let experiments = Experiments::from_compound(&map).unwrap();
        let roundtrip = experiments.to_compound();
        assert_eq!(roundtrip, map);

        let unknown: Vec<_> = experiments.iter_unknown().collect();
        assert_eq!(unknown.len(), 2);
    }

    #[test]
    fn experiments_reject_non_byte_values() {
        let mut map = CompoundTag::new();
        map.insert("caves_and_cliffs".to_string(), Tag::Int(1));
        let err = Experiments::from_compound(&map).unwrap_err();
        assert!(matches!(
            err,
            Error::UnexpectedType {
                context: "level_dat_experiments_value_type",
                expected_id,
                actual_id
            } if expected_id == TagType::Byte.id() && actual_id == TagType::Int.id()
        ));
    }
}
