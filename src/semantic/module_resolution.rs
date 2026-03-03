// Module declaration resolution
//
// This module implements:
// - Module declaration registration
// - Main module handling (module Name)
// - Submodule handling (module .name)
// - Module symbol table creation
// - Export statement validation
// - Import statement resolution

use crate::ast::NodeType;
use crate::lexer::TokenKind;
use super::{ScopeKind, SemanticAnalyzer, SemanticError, Symbol, SymbolKind};

impl SemanticAnalyzer {
    /// Visits module declaration
    /// Registers module in symbol table and creates module scope
    pub(super) fn visit_module_decl(&mut self, node_idx: usize) {
        // Check for multiple module declarations (only one per file allowed)
        if self.current_module.is_some() {
            let path_idx = self.ast.nodes[node_idx].first_child;
            if let Some(idx) = path_idx {
                if let Some(token) = self.ast.nodes[idx].token.as_ref() {
                    self.record_error(SemanticError::from_token(
                        "Only one module declaration allowed per file".to_string(),
                        token,
                    ));
                }
            }
            return;
        }

        // Extract ModulePath child
        let Some(path_idx) = self.ast.nodes[node_idx].first_child else {
            return;
        };

        // Get module path text
        let Some(module_path) = self.ast.node_text(path_idx) else {
            return;
        };
        let module_path = module_path.to_string();

        // Determine if submodule (starts with '.')
        let is_submodule = module_path.starts_with('.');
        let module_name = if is_submodule {
            module_path[1..].to_string() // Strip leading dot
        } else {
            module_path.clone()
        };

        // Create module symbol
        let symbol = Symbol::new(
            module_name.clone(),
            Some(if is_submodule { "submodule" } else { "module" }.to_string()),
            SymbolKind::Module,
        );
        self.scopes.insert(symbol);

        // Track current module context
        self.current_module = Some(module_name);
        self.is_submodule = is_submodule;

        // Enter module scope for subsequent declarations
        self.scopes.enter_scope(ScopeKind::Module);
    }

    /// Visits an export statement: export { name1, name2, ... }
    ///
    /// Validates that each exported name exists in the current scope.
    /// Records valid export names in `self.exported_symbol_names`.
    pub(super) fn visit_export_stmt(&mut self, node_idx: usize) {
        // Export → ExportList → Identifier*
        let Some(export_list_idx) = self.ast.nodes[node_idx].first_child else {
            return;
        };

        let mut current = self.ast.nodes[export_list_idx].first_child;
        while let Some(ident_idx) = current {
            if let Some(name) = self.ast.node_text(ident_idx) {
                let name = name.to_string();
                if self.scopes.lookup(&name).is_none() {
                    let error_msg = format!("Exported symbol '{}' is not defined", name);
                    if let Some(token) = self.ast.nodes[ident_idx].token.as_ref() {
                        self.record_error(SemanticError::from_token(error_msg, token));
                    } else {
                        self.record_error(SemanticError::new(error_msg, 0, 0));
                    }
                } else {
                    self.exported_symbol_names.push(name);
                }
            }
            current = self.ast.nodes[ident_idx].next_sibling;
        }
    }

    /// Visits an import statement: import { item1, item2, ... }
    ///
    /// For full module imports (`import { math }`), looks up the module in
    /// the registry and adds it to scope. Other import forms (aliased,
    /// selective, star) are silently skipped (handled in later phases).
    pub(super) fn visit_import_stmt(&mut self, node_idx: usize) {
        // If no registry is set, we are in single-file mode — skip silently.
        let registry = match &self.module_registry {
            Some(r) => r.clone(),
            None => return,
        };

        // Import → ImportList → ImportItem+
        let Some(import_list_idx) = self.ast.nodes[node_idx].first_child else {
            return;
        };

        let mut current = self.ast.nodes[import_list_idx].first_child;
        while let Some(item_idx) = current {
            self.resolve_import_item(item_idx, &registry);
            current = self.ast.nodes[item_idx].next_sibling;
        }
    }

    /// Resolves a single ImportItem node.
    ///
    /// Only full imports (`ImportItem → Identifier`) are handled here.
    /// Star, aliased, and selective imports are deferred to later phases.
    fn resolve_import_item(
        &mut self,
        item_idx: usize,
        registry: &std::rc::Rc<std::cell::RefCell<super::module_registry::ModuleRegistry>>,
    ) {
        let Some(first_child_idx) = self.ast.nodes[item_idx].first_child else {
            return;
        };

        let first_child_type = self.ast.nodes[first_child_idx].node_type;

        // Full import: ImportItem → Identifier (not star, not alias, not selective)
        if first_child_type == NodeType::Identifier {
            // Check if this is a star import (Identifier with Star token)
            if let Some(token) = self.ast.nodes[first_child_idx].token.as_ref() {
                if token.kind == TokenKind::Star {
                    // Star import — skip for now
                    return;
                }
            }

            // Full module import
            let Some(module_name) = self.ast.node_text(first_child_idx) else {
                return;
            };
            let module_name = module_name.to_string();

            let reg = registry.borrow();
            if reg.module_exists(&module_name) {
                // Add module symbol to current scope
                let symbol = Symbol::new(
                    module_name.clone(),
                    Some("module".to_string()),
                    SymbolKind::Module,
                );
                self.scopes.insert(symbol);
            } else {
                let error_msg = format!("Module '{}' not found", module_name);
                if let Some(token) = self.ast.nodes[first_child_idx].token.as_ref().cloned() {
                    self.record_error(SemanticError::from_token(error_msg, &token));
                } else {
                    self.record_error(SemanticError::new(error_msg, 0, 0));
                }
            }
        }
        // Aliased (ImportAlias), selective (ImportSelective), or star already handled above
    }

    /// Returns the list of exported symbol names collected during analysis.
    ///
    /// Called by `MultiFileAnalyzer` after per-file analysis to build the
    /// `ModuleRegistry`.
    pub fn collect_exported_names(&self) -> Vec<String> {
        self.exported_symbol_names.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::super::{SemanticAnalyzer, SemanticError, SymbolKind};
    use crate::lexer::lex;
    use crate::limits::CompilerLimits;
    use crate::parser::parse;

    fn analyze_source(source: &str) -> Result<crate::ast::Ast, Vec<SemanticError>> {
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let analyzer = SemanticAnalyzer::new(ast);
        analyzer.analyze()
    }

    /// Helper to get module info after analysis
    fn analyze_and_get_module(
        source: &str,
    ) -> (
        Option<String>,
        bool,
        Result<crate::ast::Ast, Vec<SemanticError>>,
    ) {
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let mut analyzer = SemanticAnalyzer::new(ast);

        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
        }

        let module_name = analyzer.current_module.clone();
        let is_submodule = analyzer.is_submodule;

        // Complete the analysis
        let _ = analyzer.solve_constraints();
        analyzer.apply_substitution();

        let result = if analyzer.errors.is_empty() {
            Ok(analyzer.ast)
        } else {
            Err(analyzer.errors)
        };

        (module_name, is_submodule, result)
    }

    // ========== Main Module Tests ==========

    #[test]
    fn test_module_simple() {
        let result = analyze_source("module Calculator\n");
        assert!(result.is_ok(), "Simple module declaration should succeed");
    }

    #[test]
    fn test_module_dotted_path() {
        let result = analyze_source("module math.geometry\n");
        assert!(result.is_ok(), "Dotted module path should succeed");
    }

    #[test]
    fn test_module_name_registered() {
        let (module_name, is_submodule, result) = analyze_and_get_module("module Calculator\n");
        assert!(result.is_ok());
        assert_eq!(module_name, Some("Calculator".to_string()));
        assert!(!is_submodule);
    }

    #[test]
    fn test_module_dotted_name_registered() {
        let (module_name, is_submodule, result) = analyze_and_get_module("module math.geometry\n");
        assert!(result.is_ok());
        assert_eq!(module_name, Some("math.geometry".to_string()));
        assert!(!is_submodule);
    }

    // ========== Submodule Tests ==========

    #[test]
    fn test_module_submodule() {
        let result = analyze_source("module .utils\n");
        assert!(result.is_ok(), "Submodule declaration should succeed");
    }

    #[test]
    fn test_submodule_marked_correctly() {
        let (module_name, is_submodule, result) = analyze_and_get_module("module .utils\n");
        assert!(result.is_ok());
        assert_eq!(module_name, Some("utils".to_string())); // Dot stripped
        assert!(is_submodule);
    }

    #[test]
    fn test_submodule_name_extracted() {
        let (module_name, _, result) = analyze_and_get_module("module .helpers\n");
        assert!(result.is_ok());
        // The leading dot should be stripped from the stored name
        assert_eq!(module_name, Some("helpers".to_string()));
    }

    // ========== Module with Declarations Tests ==========

    #[test]
    fn test_module_with_function() {
        let source = r#"module Calculator

add: (x Number, y Number) Number {
    return x
}
"#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Module with function should succeed");
    }

    #[test]
    fn test_module_with_variable() {
        let source = r#"module Config

version: 1
"#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Module with variable should succeed");
    }

    #[test]
    fn test_module_with_type() {
        let source = r#"module Types

type UserId: Number
"#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Module with type should succeed");
    }

    #[test]
    fn test_module_complete() {
        let source = r#"module Calculator

type CalcResult: Number

version: 1

add: (x Number, y Number) Number {
    return x
}
"#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Complete module should succeed");
    }

    // ========== Error Cases ==========

    #[test]
    fn test_module_multiple_error() {
        let source = r#"module First
module Second
"#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Multiple module declarations should fail");
        let errors = result.unwrap_err();
        assert!(
            errors[0]
                .message
                .contains("Only one module declaration allowed")
        );
    }

    #[test]
    fn test_module_multiple_with_declarations() {
        let source = r#"module First

x: 1

module Second
"#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Second module declaration should fail");
        let errors = result.unwrap_err();
        assert!(
            errors[0]
                .message
                .contains("Only one module declaration allowed")
        );
    }

    // ========== Symbol Table Tests ==========

    #[test]
    fn test_module_symbol_in_table() {
        let limits = CompilerLimits::default();
        let tokens = lex("module Calculator\n", &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let mut analyzer = SemanticAnalyzer::new(ast);

        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
        }

        // Module should be in symbol table
        let symbol = analyzer.scopes.lookup("Calculator");
        assert!(symbol.is_some(), "Module symbol should be registered");
        assert_eq!(symbol.unwrap().kind, SymbolKind::Module);
        assert_eq!(symbol.unwrap().type_name, Some("module".to_string()));
    }

    #[test]
    fn test_submodule_symbol_in_table() {
        let limits = CompilerLimits::default();
        let tokens = lex("module .utils\n", &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let mut analyzer = SemanticAnalyzer::new(ast);

        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
        }

        // Submodule should be in symbol table with stripped name
        let symbol = analyzer.scopes.lookup("utils");
        assert!(symbol.is_some(), "Submodule symbol should be registered");
        assert_eq!(symbol.unwrap().kind, SymbolKind::Module);
        assert_eq!(symbol.unwrap().type_name, Some("submodule".to_string()));
    }

    // ========== Scope Tests ==========

    #[test]
    fn test_module_creates_scope() {
        let limits = CompilerLimits::default();
        let tokens = lex("module Calculator\n", &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let mut analyzer = SemanticAnalyzer::new(ast);

        // Before analysis, depth is 0 (global)
        assert_eq!(analyzer.scopes.depth(), 0);

        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
        }

        // After visiting module, we should be in module scope (depth 1)
        assert_eq!(analyzer.scopes.depth(), 1);
    }

    #[test]
    fn test_declarations_in_module_scope() {
        let source = r#"module Calculator

x: 42
"#;
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let mut analyzer = SemanticAnalyzer::new(ast);

        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
        }

        // Variable x should be in the module scope
        let symbol = analyzer.scopes.lookup("x");
        assert!(
            symbol.is_some(),
            "Variable should be visible in module scope"
        );
    }

    // ========== Export Statement Tests ==========

    #[test]
    fn test_export_stmt_valid() {
        // Export an existing symbol — should succeed with no errors
        let source = "x: 42\nexport { x }\n";
        let result = analyze_source(source);
        assert!(result.is_ok(), "Export of defined symbol should succeed: {:?}", result);
    }

    #[test]
    fn test_export_stmt_undefined_symbol() {
        // Export a non-existent symbol — should produce an error
        let source = "export { undeclared }\n";
        let result = analyze_source(source);
        assert!(result.is_err(), "Export of undefined symbol should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Exported symbol 'undeclared' is not defined")),
            "Error should mention the missing symbol"
        );
    }

    #[test]
    fn test_export_stmt_multiple() {
        // Export several symbols, all defined
        let source = "a: 1\nb: 2\nc: 3\nexport { a, b, c }\n";
        let result = analyze_source(source);
        assert!(result.is_ok(), "Export of multiple defined symbols should succeed: {:?}", result);
    }

    // ========== Import Statement Tests (no registry = single-file mode) ==========

    #[test]
    fn test_import_stmt_no_registry() {
        // Without a registry, import should be silently ignored (no error)
        let source = "import { math }\n";
        let result = analyze_source(source);
        assert!(result.is_ok(), "Import without registry should be silently ignored: {:?}", result);
    }

    // ========== Import Statement Tests (with registry) ==========

    #[test]
    fn test_import_stmt_with_registry_found() {
        use super::super::module_registry::{ModuleExportedSymbol, ModuleRegistry};
        use std::cell::RefCell;
        use std::rc::Rc;

        let source = "import { math }\n";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();

        let mut registry = ModuleRegistry::new();
        registry.register_module("math".to_string());
        registry.add_export(
            "math",
            ModuleExportedSymbol::new("add".to_string(), SymbolKind::Function),
        );
        let registry_rc = Rc::new(RefCell::new(registry));

        let analyzer = SemanticAnalyzer::new(ast).with_module_registry(registry_rc);
        let result = analyzer.analyze();
        assert!(result.is_ok(), "Import of known module should succeed: {:?}", result);
    }

    #[test]
    fn test_import_stmt_with_registry_not_found() {
        use super::super::module_registry::ModuleRegistry;
        use std::cell::RefCell;
        use std::rc::Rc;

        let source = "import { unknown_module }\n";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();

        let registry = ModuleRegistry::new(); // empty — no modules registered
        let registry_rc = Rc::new(RefCell::new(registry));

        let analyzer = SemanticAnalyzer::new(ast).with_module_registry(registry_rc);
        let result = analyzer.analyze();
        assert!(result.is_err(), "Import of unknown module should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Module 'unknown_module' not found")),
            "Error should mention the missing module"
        );
    }
}
