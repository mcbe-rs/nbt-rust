use std::fmt;
use std::io;

use thiserror::Error as ThisError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorContext {
    pub op: &'static str,
    pub offset: usize,
    pub field: Option<&'static str>,
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(field) = self.field {
            write!(
                f,
                "{} failed at offset={} field={}",
                self.op, self.offset, field
            )
        } else {
            write!(f, "{} failed at offset={}", self.op, self.offset)
        }
    }
}

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("i/o error: {0}")]
    Io(#[from] io::Error),
    #[error("{context}: {source}")]
    Context {
        context: ErrorContext,
        #[source]
        source: Box<Error>,
    },
    #[error("serde error: {message}")]
    Serde { message: String },
    #[error("unknown tag id: {id}")]
    UnknownTag { id: u8 },
    #[error("invalid root tag type id: {id}")]
    InvalidRoot { id: u8 },
    #[error("invalid header ({detail}): expected={expected:?}, actual={actual:?}")]
    InvalidHeader {
        detail: &'static str,
        expected: Option<u32>,
        actual: Option<u32>,
    },
    #[error("maximum depth exceeded: depth={depth}, max_depth={max_depth}")]
    DepthExceeded { depth: usize, max_depth: usize },
    #[error("size exceeded for {field}: max={max}, actual={actual}")]
    SizeExceeded {
        field: &'static str,
        max: usize,
        actual: usize,
    },
    #[error("trailing payload bytes: unread={unread}")]
    TrailingPayloadBytes { unread: usize },
    #[error("unexpected TAG_End payload")]
    UnexpectedEndTagPayload,
    #[error("invalid UTF-8 for {field}")]
    InvalidUtf8 { field: &'static str },
    #[error("invalid list header: element_type_id={element_type_id}, length={length}")]
    InvalidListHeader { element_type_id: u8, length: usize },
    #[error("unexpected type ({context}): expected_id={expected_id}, actual_id={actual_id}")]
    UnexpectedType {
        context: &'static str,
        expected_id: u8,
        actual_id: u8,
    },
    #[error("invalid structure shape: {detail}")]
    InvalidStructureShape { detail: &'static str },
    #[error("invalid palette index: index={index}, palette_len={palette_len}")]
    InvalidPaletteIndex { index: i32, palette_len: usize },
    #[error("negative length for {field}: {value}")]
    NegativeLength { field: &'static str, value: i32 },
    #[error("length overflow for {field}: max={max}, actual={actual}")]
    LengthOverflow {
        field: &'static str,
        max: usize,
        actual: usize,
    },
    #[error("invalid varint: {detail}")]
    InvalidVarint { detail: &'static str },
}

impl Error {
    pub fn with_context(
        self,
        op: &'static str,
        offset: usize,
        field: Option<&'static str>,
    ) -> Self {
        Self::Context {
            context: ErrorContext { op, offset, field },
            source: Box::new(self),
        }
    }

    pub fn innermost(&self) -> &Error {
        match self {
            Error::Context { source, .. } => source.innermost(),
            _ => self,
        }
    }

    pub fn has_context(&self, op: &'static str, field: Option<&'static str>) -> bool {
        match self {
            Error::Context { context, source } => {
                if context.op == op && context.field == field {
                    true
                } else {
                    source.has_context(op, field)
                }
            }
            _ => false,
        }
    }
}
