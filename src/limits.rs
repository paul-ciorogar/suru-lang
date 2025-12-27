// Compiler safety limits module
//
// Provides configurable resource limits to prevent:
// - Stack overflow from deeply nested expressions
// - Memory exhaustion from very large source files
// - Denial of service from pathological input
//
// All limits have sensible defaults and can be overridden via project.toml

use serde::Deserialize;
use std::fs;
use std::path::Path;

/// Compiler safety limits with permissive defaults
#[derive(Debug, Clone)]
pub struct CompilerLimits {
    // Lexer limits
    pub max_input_size: usize,        // Maximum source file size in bytes
    pub max_token_count: usize,       // Maximum number of tokens per file
    pub max_identifier_length: usize, // Maximum identifier length in bytes
    pub max_string_length: usize,     // Maximum string literal length in bytes
    pub max_comment_length: usize,    // Maximum comment length in bytes

    // Parser limits
    pub max_expr_depth: usize, // Maximum expression recursion depth

    // AST limits
    pub max_ast_nodes: usize, // Maximum AST nodes per file
}

// Default limits (permissive for developer productivity)
impl Default for CompilerLimits {
    fn default() -> Self {
        Self {
            max_input_size: 10_000_000,    // 10 MB
            max_token_count: 100_000,      // 100k tokens
            max_identifier_length: 1_000,  // 1k bytes
            max_string_length: 10_000_000, // 10 MB
            max_comment_length: 100_000,   // 100k bytes
            max_expr_depth: 256,
            max_ast_nodes: 1_000_000, // 1M nodes
        }
    }
}

impl CompilerLimits {
    /// Create with default limits
    pub fn new() -> Self {
        Self::default()
    }

    /// Load limits from project.toml, falling back to defaults
    ///
    /// Returns error only if TOML is malformed, not if file is missing
    pub fn from_project_toml<P: AsRef<Path>>(path: P) -> Result<Self, LimitError> {
        let path = path.as_ref();

        // If file doesn't exist, use defaults
        if !path.exists() {
            return Ok(Self::default());
        }

        // Read file
        let content = fs::read_to_string(path).map_err(|e| LimitError {
            message: format!("Failed to read {}: {}", path.display(), e),
        })?;

        // Parse TOML
        let config: ProjectConfig = toml::from_str(&content).map_err(|e| LimitError {
            message: format!("Failed to parse {}: {}", path.display(), e),
        })?;

        // Merge with defaults (only override specified values)
        let mut limits = Self::default();

        if let Some(limits_config) = config.limits {
            if let Some(v) = limits_config.max_input_size {
                limits.max_input_size = v;
            }
            if let Some(v) = limits_config.max_token_count {
                limits.max_token_count = v;
            }
            if let Some(v) = limits_config.max_identifier_length {
                limits.max_identifier_length = v;
            }
            if let Some(v) = limits_config.max_string_length {
                limits.max_string_length = v;
            }
            if let Some(v) = limits_config.max_comment_length {
                limits.max_comment_length = v;
            }
            if let Some(v) = limits_config.max_expr_depth {
                limits.max_expr_depth = v;
            }
            if let Some(v) = limits_config.max_ast_nodes {
                limits.max_ast_nodes = v;
            }
        }

        Ok(limits)
    }

    /// Validate that all limits are reasonable (positive, not absurdly large)
    pub fn validate(&self) -> Result<(), LimitError> {
        const MAX_REASONABLE: usize = 100_000_000; // 100 MB

        if self.max_input_size == 0 || self.max_input_size > MAX_REASONABLE {
            return Err(LimitError::invalid("max_input_size", self.max_input_size));
        }

        if self.max_token_count == 0 {
            return Err(LimitError::invalid("max_token_count", self.max_token_count));
        }

        if self.max_identifier_length == 0 || self.max_identifier_length > 100_000 {
            return Err(LimitError::invalid(
                "max_identifier_length",
                self.max_identifier_length,
            ));
        }

        if self.max_string_length == 0 || self.max_string_length > MAX_REASONABLE {
            return Err(LimitError::invalid(
                "max_string_length",
                self.max_string_length,
            ));
        }

        if self.max_comment_length == 0 || self.max_comment_length > MAX_REASONABLE {
            return Err(LimitError::invalid(
                "max_comment_length",
                self.max_comment_length,
            ));
        }

        if self.max_expr_depth == 0 || self.max_expr_depth > 10_000 {
            return Err(LimitError::invalid("max_expr_depth", self.max_expr_depth));
        }

        if self.max_ast_nodes == 0 || self.max_ast_nodes > 10_000_000 {
            return Err(LimitError::invalid("max_ast_nodes", self.max_ast_nodes));
        }

        Ok(())
    }
}

/// TOML configuration structures for deserialization
#[derive(Debug, Deserialize)]
struct ProjectConfig {
    limits: Option<LimitsConfig>,
}

#[derive(Debug, Deserialize)]
struct LimitsConfig {
    max_input_size: Option<usize>,
    max_token_count: Option<usize>,
    max_identifier_length: Option<usize>,
    max_string_length: Option<usize>,
    max_comment_length: Option<usize>,
    max_expr_depth: Option<usize>,
    max_ast_nodes: Option<usize>,
}

/// Error type for limit validation and loading
#[derive(Debug, Clone)]
pub struct LimitError {
    pub message: String,
}

impl LimitError {
    fn invalid(name: &str, value: usize) -> Self {
        Self {
            message: format!(
                "Invalid limit '{}': {} (must be positive and reasonable)",
                name, value
            ),
        }
    }
}

impl std::fmt::Display for LimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Limit error: {}", self.message)
    }
}

impl std::error::Error for LimitError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_limits_are_reasonable() {
        let limits = CompilerLimits::default();
        assert!(limits.validate().is_ok());
    }

    #[test]
    fn test_default_values() {
        let limits = CompilerLimits::default();
        assert_eq!(limits.max_input_size, 10_000_000); // 10 MB
        assert_eq!(limits.max_token_count, 100_000);
        assert_eq!(limits.max_identifier_length, 1_000);
        assert_eq!(limits.max_string_length, 10_000_000);
        assert_eq!(limits.max_comment_length, 100_000);
        assert_eq!(limits.max_expr_depth, 256);
        assert_eq!(limits.max_ast_nodes, 1_000_000);
    }

    #[test]
    fn test_validation_catches_zero_values() {
        let mut limits = CompilerLimits::default();
        limits.max_input_size = 0;
        assert!(limits.validate().is_err());

        limits = CompilerLimits::default();
        limits.max_token_count = 0;
        assert!(limits.validate().is_err());

        limits = CompilerLimits::default();
        limits.max_identifier_length = 0;
        assert!(limits.validate().is_err());
    }

    #[test]
    fn test_validation_catches_too_large_values() {
        let mut limits = CompilerLimits::default();
        limits.max_input_size = 200_000_000; // 200 MB - too large
        assert!(limits.validate().is_err());

        limits = CompilerLimits::default();
        limits.max_expr_depth = 20_000; // Too deep
        assert!(limits.validate().is_err());
    }

    #[test]
    fn test_missing_file_uses_defaults() {
        let limits = CompilerLimits::from_project_toml("nonexistent.toml").unwrap();
        assert_eq!(limits.max_input_size, 10_000_000); // 10 MB
        assert_eq!(limits.max_token_count, 100_000);
    }

    #[test]
    fn test_partial_override() {
        // Create temporary TOML file
        let toml_content = r#"
[limits]
max_input_size = 2000000
max_expr_depth = 128
"#;
        let temp_path = "/tmp/test_limits.toml";
        fs::write(temp_path, toml_content).unwrap();

        let limits = CompilerLimits::from_project_toml(temp_path).unwrap();
        assert_eq!(limits.max_input_size, 2_000_000); // Overridden
        assert_eq!(limits.max_expr_depth, 128); // Overridden
        assert_eq!(limits.max_token_count, 100_000); // Default
        assert_eq!(limits.max_identifier_length, 1_000); // Default

        // Cleanup
        let _ = fs::remove_file(temp_path);
    }

    #[test]
    fn test_malformed_toml_returns_error() {
        let toml_content = "this is not valid toml {{{";
        let temp_path = "/tmp/test_malformed.toml";
        fs::write(temp_path, toml_content).unwrap();

        let result = CompilerLimits::from_project_toml(temp_path);
        assert!(result.is_err());

        // Cleanup
        let _ = fs::remove_file(temp_path);
    }
}
