#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NbtLimits {
    pub max_depth: usize,
    pub max_read_bytes: usize,
    pub max_string_len: usize,
    pub max_array_len: usize,
    pub max_list_len: usize,
    pub max_compound_entries: usize,
}

impl NbtLimits {
    pub const fn new(
        max_depth: usize,
        max_read_bytes: usize,
        max_string_len: usize,
        max_array_len: usize,
        max_list_len: usize,
        max_compound_entries: usize,
    ) -> Self {
        Self {
            max_depth,
            max_read_bytes,
            max_string_len,
            max_array_len,
            max_list_len,
            max_compound_entries,
        }
    }

    pub const fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    pub const fn with_max_read_bytes(mut self, max_read_bytes: usize) -> Self {
        self.max_read_bytes = max_read_bytes;
        self
    }

    pub const fn with_max_string_len(mut self, max_string_len: usize) -> Self {
        self.max_string_len = max_string_len;
        self
    }

    pub const fn with_max_array_len(mut self, max_array_len: usize) -> Self {
        self.max_array_len = max_array_len;
        self
    }

    pub const fn with_max_list_len(mut self, max_list_len: usize) -> Self {
        self.max_list_len = max_list_len;
        self
    }

    pub const fn with_max_compound_entries(mut self, max_compound_entries: usize) -> Self {
        self.max_compound_entries = max_compound_entries;
        self
    }
}

impl Default for NbtLimits {
    fn default() -> Self {
        Self {
            max_depth: 512,
            max_read_bytes: 16 * 1024 * 1024,
            max_string_len: 1024 * 1024,
            max_array_len: 4 * 1024 * 1024,
            max_list_len: 1024 * 1024,
            max_compound_entries: 1024 * 1024,
        }
    }
}
