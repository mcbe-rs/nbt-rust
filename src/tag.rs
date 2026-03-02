use indexmap::IndexMap;

use crate::error::{Error, Result};

pub type CompoundTag = IndexMap<String, Tag>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TagType {
    End = 0,
    Byte = 1,
    Short = 2,
    Int = 3,
    Long = 4,
    Float = 5,
    Double = 6,
    ByteArray = 7,
    String = 8,
    List = 9,
    Compound = 10,
    IntArray = 11,
    LongArray = 12,
}

impl TagType {
    pub const fn id(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for TagType {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self> {
        let tag_type = match value {
            0 => Self::End,
            1 => Self::Byte,
            2 => Self::Short,
            3 => Self::Int,
            4 => Self::Long,
            5 => Self::Float,
            6 => Self::Double,
            7 => Self::ByteArray,
            8 => Self::String,
            9 => Self::List,
            10 => Self::Compound,
            11 => Self::IntArray,
            12 => Self::LongArray,
            _ => return Err(Error::UnknownTag { id: value }),
        };
        Ok(tag_type)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListTag {
    pub element_type: TagType,
    pub elements: Vec<Tag>,
}

impl ListTag {
    pub fn new(element_type: TagType, elements: Vec<Tag>) -> Result<Self> {
        let list = Self {
            element_type,
            elements,
        };
        list.validate()?;
        Ok(list)
    }

    pub fn empty(element_type: TagType) -> Self {
        Self {
            element_type,
            elements: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.element_type == TagType::End && !self.elements.is_empty() {
            return Err(Error::InvalidListHeader {
                element_type_id: self.element_type.id(),
                length: self.elements.len(),
            });
        }

        for element in &self.elements {
            let actual = element.tag_type();
            if actual != self.element_type {
                return Err(Error::UnexpectedType {
                    context: "list_element_type",
                    expected_id: self.element_type.id(),
                    actual_id: actual.id(),
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tag {
    /// Marker value for TAG_End (id=0).
    ///
    /// This is modeled explicitly for completeness, but TAG_End is not a
    /// normal payload value in NBT streams.
    End,
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<u8>),
    String(String),
    List(ListTag),
    Compound(CompoundTag),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

impl Tag {
    pub fn tag_type(&self) -> TagType {
        match self {
            Tag::End => TagType::End,
            Tag::Byte(_) => TagType::Byte,
            Tag::Short(_) => TagType::Short,
            Tag::Int(_) => TagType::Int,
            Tag::Long(_) => TagType::Long,
            Tag::Float(_) => TagType::Float,
            Tag::Double(_) => TagType::Double,
            Tag::ByteArray(_) => TagType::ByteArray,
            Tag::String(_) => TagType::String,
            Tag::List(_) => TagType::List,
            Tag::Compound(_) => TagType::Compound,
            Tag::IntArray(_) => TagType::IntArray,
            Tag::LongArray(_) => TagType::LongArray,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_tag_maps_to_end_tag_type() {
        assert_eq!(Tag::End.tag_type(), TagType::End);
    }
}
