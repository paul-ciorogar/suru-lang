// Module registry for cross-file module export tracking.
//
// Stores exported symbol names per module so that SemanticAnalyzer can
// resolve imports across files during multi-file analysis.

use std::collections::{HashMap, HashSet};

use super::SymbolKind;

/// A single exported symbol from a module
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleExportedSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub type_name: Option<String>,
}

impl ModuleExportedSymbol {
    pub fn new(name: String, kind: SymbolKind) -> Self {
        ModuleExportedSymbol { name, kind, type_name: None }
    }

    pub fn with_type_name(mut self, type_name: String) -> Self {
        self.type_name = Some(type_name);
        self
    }
}

/// Registry mapping module names to their exported symbols.
///
/// Used by `MultiFileAnalyzer` to build a shared view of all modules and by
/// `SemanticAnalyzer` to resolve import statements at analysis time.
pub struct ModuleRegistry {
    modules: HashMap<String, Vec<ModuleExportedSymbol>>,
    submodules: HashSet<String>,
}

impl ModuleRegistry {
    /// Creates a new empty registry
    pub fn new() -> Self {
        ModuleRegistry { modules: HashMap::new(), submodules: HashSet::new() }
    }

    /// Registers an empty module.
    /// If the module already exists, this is a no-op.
    pub fn register_module(&mut self, name: String) {
        self.modules.entry(name).or_insert_with(Vec::new);
    }

    /// Registers a module and marks it as a submodule (declared with `module .name`).
    /// If the module is already registered, this is a no-op for the exports but
    /// still marks it as a submodule.
    pub fn register_submodule(&mut self, name: String) {
        self.modules.entry(name.clone()).or_insert_with(Vec::new);
        self.submodules.insert(name);
    }

    /// Returns true if the named module was declared as a submodule (`module .name`).
    pub fn is_submodule(&self, name: &str) -> bool {
        self.submodules.contains(name)
    }

    /// Returns true if a module with the given name is registered
    pub fn module_exists(&self, name: &str) -> bool {
        self.modules.contains_key(name)
    }

    /// Adds an exported symbol to a registered module.
    /// Returns true on success, false if the module does not exist.
    pub fn add_export(&mut self, module_name: &str, symbol: ModuleExportedSymbol) -> bool {
        if let Some(exports) = self.modules.get_mut(module_name) {
            exports.push(symbol);
            true
        } else {
            false
        }
    }

    /// Returns the exported symbols for a module, or None if not found.
    pub fn get_module_exports(&self, module_name: &str) -> Option<&[ModuleExportedSymbol]> {
        self.modules.get(module_name).map(|v| v.as_slice())
    }

    /// Returns a specific exported symbol by module name and symbol name.
    pub fn get_symbol(&self, module: &str, name: &str) -> Option<&ModuleExportedSymbol> {
        self.modules
            .get(module)?
            .iter()
            .find(|s| s.name == name)
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::SymbolKind;

    #[test]
    fn test_registry_new_empty() {
        let registry = ModuleRegistry::new();
        assert!(!registry.module_exists("math"));
        assert!(registry.get_module_exports("math").is_none());
    }

    #[test]
    fn test_registry_register_module() {
        let mut registry = ModuleRegistry::new();
        registry.register_module("math".to_string());
        assert!(registry.module_exists("math"));
    }

    #[test]
    fn test_registry_module_exists() {
        let mut registry = ModuleRegistry::new();
        assert!(!registry.module_exists("math"));
        registry.register_module("math".to_string());
        assert!(registry.module_exists("math"));
        assert!(!registry.module_exists("other"));
    }

    #[test]
    fn test_registry_module_not_found() {
        let registry = ModuleRegistry::new();
        assert!(registry.get_module_exports("nonexistent").is_none());
        assert!(registry.get_symbol("nonexistent", "add").is_none());
    }

    #[test]
    fn test_registry_add_export() {
        let mut registry = ModuleRegistry::new();
        registry.register_module("math".to_string());

        let symbol = ModuleExportedSymbol::new("add".to_string(), SymbolKind::Function);
        assert!(registry.add_export("math", symbol));

        let exports = registry.get_module_exports("math").unwrap();
        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].name, "add");
        assert_eq!(exports[0].kind, SymbolKind::Function);
    }

    #[test]
    fn test_registry_get_symbol() {
        let mut registry = ModuleRegistry::new();
        registry.register_module("math".to_string());

        registry.add_export(
            "math",
            ModuleExportedSymbol::new("add".to_string(), SymbolKind::Function),
        );
        registry.add_export(
            "math",
            ModuleExportedSymbol::new("pi".to_string(), SymbolKind::Variable),
        );

        let sym = registry.get_symbol("math", "add").unwrap();
        assert_eq!(sym.name, "add");
        assert_eq!(sym.kind, SymbolKind::Function);

        let sym2 = registry.get_symbol("math", "pi").unwrap();
        assert_eq!(sym2.kind, SymbolKind::Variable);

        assert!(registry.get_symbol("math", "missing").is_none());
    }

    #[test]
    fn test_registry_add_export_unknown_module_returns_false() {
        let mut registry = ModuleRegistry::new();
        let symbol = ModuleExportedSymbol::new("add".to_string(), SymbolKind::Function);
        assert!(!registry.add_export("nonexistent", symbol));
    }

    #[test]
    fn test_registry_register_module_idempotent() {
        let mut registry = ModuleRegistry::new();
        registry.register_module("math".to_string());
        registry.add_export(
            "math",
            ModuleExportedSymbol::new("add".to_string(), SymbolKind::Function),
        );
        // Registering again should not clear existing exports
        registry.register_module("math".to_string());
        let exports = registry.get_module_exports("math").unwrap();
        assert_eq!(exports.len(), 1, "Re-registering should not clear exports");
    }

    #[test]
    fn test_registry_register_submodule() {
        let mut registry = ModuleRegistry::new();
        registry.register_submodule("utils".to_string());
        assert!(registry.module_exists("utils"), "Submodule should be registered as a module");
        assert!(registry.is_submodule("utils"), "Submodule should be marked as a submodule");
    }

    #[test]
    fn test_registry_is_submodule() {
        let mut registry = ModuleRegistry::new();
        registry.register_submodule("utils".to_string());
        registry.register_module("math".to_string());
        assert!(registry.is_submodule("utils"));
        assert!(!registry.is_submodule("math"), "Regular module should not be a submodule");
    }

    #[test]
    fn test_registry_non_submodule_returns_false() {
        let registry = ModuleRegistry::new();
        assert!(!registry.is_submodule("nonexistent"), "Unregistered name should not be a submodule");
    }
}
