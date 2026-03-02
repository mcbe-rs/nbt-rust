use crate::limits::NbtLimits;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParseMode {
    #[default]
    Strict,
    Compatible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NbtReadConfig {
    pub limits: NbtLimits,
    pub parse_mode: ParseMode,
}

impl NbtReadConfig {
    pub const fn new(limits: NbtLimits, parse_mode: ParseMode) -> Self {
        Self { limits, parse_mode }
    }

    pub const fn strict(limits: NbtLimits) -> Self {
        Self::new(limits, ParseMode::Strict)
    }

    pub const fn compatible(limits: NbtLimits) -> Self {
        Self::new(limits, ParseMode::Compatible)
    }

    pub const fn with_parse_mode(mut self, parse_mode: ParseMode) -> Self {
        self.parse_mode = parse_mode;
        self
    }

    pub const fn with_limits(mut self, limits: NbtLimits) -> Self {
        self.limits = limits;
        self
    }
}

impl Default for NbtReadConfig {
    fn default() -> Self {
        Self::strict(NbtLimits::default())
    }
}
