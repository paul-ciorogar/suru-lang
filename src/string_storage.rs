/// Unique identifier for an interned string
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StringId(usize);

/// String storage for deduplicating identifiers and string literals
/// Uses Vec-only implementation with linear search for simplicity
#[derive(Debug, Clone)]
pub struct StringStorage {
    strings: Vec<String>,
}

impl StringStorage {
    /// Create a new empty string storage
    pub fn new() -> Self {
        Self {
            strings: Vec::new(),
        }
    }

    /// Intern a string and return its unique ID
    /// If the string already exists, returns existing ID (via linear search)
    /// If not found, adds the string and returns new ID
    pub fn intern(&mut self, s: &str) -> StringId {
        // Linear search for existing string
        for (idx, existing) in self.strings.iter().enumerate() {
            if existing == s {
                return StringId(idx);
            }
        }

        // Not found, add new string
        let id = StringId(self.strings.len());
        self.strings.push(s.to_string());
        id
    }

    /// Get string content by ID
    pub fn resolve(&self, id: StringId) -> &str {
        &self.strings[id.0]
    }

    /// Get number of unique strings stored
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Check if storage is empty
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
}

impl Default for StringStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_interning() {
        let mut storage = StringStorage::new();

        let id1 = storage.intern("hello");
        let id2 = storage.intern("world");
        let id3 = storage.intern("hello"); // Same as id1

        assert_eq!(id1, id3);
        assert_ne!(id1, id2);
        assert_eq!(storage.resolve(id1), "hello");
        assert_eq!(storage.resolve(id2), "world");
        assert_eq!(storage.resolve(id3), "hello");
    }

    #[test]
    fn test_deduplication() {
        let mut storage = StringStorage::new();

        storage.intern("foo");
        storage.intern("bar");
        storage.intern("foo"); // Duplicate
        storage.intern("baz");
        storage.intern("foo"); // Another duplicate
        storage.intern("bar"); // Another duplicate

        // Only 3 unique strings should be stored
        assert_eq!(storage.len(), 3);

        // Verify all strings are accessible
        let foo_id = storage.intern("foo");
        let bar_id = storage.intern("bar");
        let baz_id = storage.intern("baz");

        assert_eq!(storage.resolve(foo_id), "foo");
        assert_eq!(storage.resolve(bar_id), "bar");
        assert_eq!(storage.resolve(baz_id), "baz");
    }

    #[test]
    fn test_empty_strings() {
        let mut storage = StringStorage::new();

        let id1 = storage.intern("");
        let id2 = storage.intern("");

        assert_eq!(id1, id2);
        assert_eq!(storage.resolve(id1), "");
        assert_eq!(storage.len(), 1);
    }
}
