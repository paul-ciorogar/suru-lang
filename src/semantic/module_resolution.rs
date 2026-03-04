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
                } else if self.exported_symbol_names.contains(&name) {
                    let error_msg = format!("Symbol '{}' is exported more than once", name);
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
    /// Handles all four import forms:
    /// - Full:     `ImportItem → Identifier("math")`
    /// - Aliased:  `ImportItem → ImportAlias("m"), Identifier("math")`
    /// - Star:     `ImportItem → Identifier("*"), Identifier("math")`
    /// - Selective:`ImportItem → ImportSelective(→ ImportSelector*), Identifier("math")`
    fn resolve_import_item(
        &mut self,
        item_idx: usize,
        registry: &std::rc::Rc<std::cell::RefCell<super::module_registry::ModuleRegistry>>,
    ) {
        let Some(first_child_idx) = self.ast.nodes[item_idx].first_child else {
            return;
        };

        let first_child_type = self.ast.nodes[first_child_idx].node_type;

        match first_child_type {
            NodeType::Identifier => {
                let is_star = self.ast.nodes[first_child_idx]
                    .token
                    .as_ref()
                    .map(|t| t.kind == TokenKind::Star)
                    .unwrap_or(false);

                if is_star {
                    self.resolve_star_import(first_child_idx, registry);
                } else {
                    self.resolve_full_import(first_child_idx, registry);
                }
            }
            NodeType::ImportAlias => {
                self.resolve_aliased_import(first_child_idx, registry);
            }
            NodeType::ImportSelective => {
                self.resolve_selective_import(first_child_idx, registry);
            }
            _ => {}
        }
    }

    /// Handles: `import { math }` — adds module symbol to scope.
    fn resolve_full_import(
        &mut self,
        ident_idx: usize,
        registry: &std::rc::Rc<std::cell::RefCell<super::module_registry::ModuleRegistry>>,
    ) {
        let Some(module_name) = self.ast.node_text(ident_idx) else {
            return;
        };
        let module_name = module_name.to_string();

        let reg = registry.borrow();
        if reg.module_exists(&module_name) {
            let accessible = self.is_submodule_accessible(&module_name, &reg);
            if !accessible {
                let error_msg = format!("Cannot import submodule '{}' from outside its package", module_name);
                let token = self.ast.nodes[ident_idx].token.as_ref().cloned();
                drop(reg);
                if let Some(token) = token {
                    self.record_error(SemanticError::from_token(error_msg, &token));
                } else {
                    self.record_error(SemanticError::new(error_msg, 0, 0));
                }
                return;
            }
            let symbol = Symbol::new(module_name.clone(), Some("module".to_string()), SymbolKind::Module);
            self.scopes.insert(symbol);
        } else {
            let error_msg = format!("Module '{}' not found", module_name);
            let token = self.ast.nodes[ident_idx].token.as_ref().cloned();
            drop(reg);
            if let Some(token) = token {
                self.record_error(SemanticError::from_token(error_msg, &token));
            } else {
                self.record_error(SemanticError::new(error_msg, 0, 0));
            }
        }
    }

    /// Handles: `import { m: math }` — adds alias `m` (not `math`) to scope.
    fn resolve_aliased_import(
        &mut self,
        alias_idx: usize,
        registry: &std::rc::Rc<std::cell::RefCell<super::module_registry::ModuleRegistry>>,
    ) {
        let Some(alias_name) = self.ast.node_text(alias_idx).map(|s| s.to_string()) else {
            return;
        };
        let Some(module_ident_idx) = self.ast.nodes[alias_idx].next_sibling else {
            return;
        };
        let Some(module_name) = self.ast.node_text(module_ident_idx).map(|s| s.to_string()) else {
            return;
        };

        let reg = registry.borrow();
        if reg.module_exists(&module_name) {
            let accessible = self.is_submodule_accessible(&module_name, &reg);
            if !accessible {
                let error_msg = format!("Cannot import submodule '{}' from outside its package", module_name);
                let token = self.ast.nodes[module_ident_idx].token.as_ref().cloned();
                drop(reg);
                if let Some(token) = token {
                    self.record_error(SemanticError::from_token(error_msg, &token));
                } else {
                    self.record_error(SemanticError::new(error_msg, 0, 0));
                }
                return;
            }
            drop(reg);
            let symbol = Symbol::new(alias_name, Some("module".to_string()), SymbolKind::Module);
            self.scopes.insert(symbol);
        } else {
            let error_msg = format!("Module '{}' not found", module_name);
            let token = self.ast.nodes[module_ident_idx].token.as_ref().cloned();
            drop(reg);
            if let Some(token) = token {
                self.record_error(SemanticError::from_token(error_msg, &token));
            } else {
                self.record_error(SemanticError::new(error_msg, 0, 0));
            }
        }
    }

    /// Handles: `import { *: math }` — adds all exported symbols from `math` to scope.
    fn resolve_star_import(
        &mut self,
        star_idx: usize,
        registry: &std::rc::Rc<std::cell::RefCell<super::module_registry::ModuleRegistry>>,
    ) {
        let Some(module_ident_idx) = self.ast.nodes[star_idx].next_sibling else {
            return;
        };
        let Some(module_name) = self.ast.node_text(module_ident_idx).map(|s| s.to_string()) else {
            return;
        };

        let reg = registry.borrow();
        let accessible = self.is_submodule_accessible(&module_name, &reg);
        let symbols: Option<Vec<Symbol>> = reg
            .get_module_exports(&module_name)
            .map(|exports| {
                exports
                    .iter()
                    .map(|exp| Symbol::new(exp.name.clone(), exp.type_name.clone(), exp.kind))
                    .collect()
            });
        drop(reg);

        match symbols {
            Some(syms) => {
                if !accessible {
                    let error_msg = format!("Cannot import submodule '{}' from outside its package", module_name);
                    let token = self.ast.nodes[module_ident_idx].token.as_ref().cloned();
                    if let Some(token) = token {
                        self.record_error(SemanticError::from_token(error_msg, &token));
                    } else {
                        self.record_error(SemanticError::new(error_msg, 0, 0));
                    }
                    return;
                }
                for sym in syms {
                    self.scopes.insert(sym);
                }
            }
            None => {
                let error_msg = format!("Module '{}' not found", module_name);
                let token = self.ast.nodes[module_ident_idx].token.as_ref().cloned();
                if let Some(token) = token {
                    self.record_error(SemanticError::from_token(error_msg, &token));
                } else {
                    self.record_error(SemanticError::new(error_msg, 0, 0));
                }
            }
        }
    }

    /// Handles: `import { {sin, cos}: math }` — adds selected symbols from `math` to scope.
    fn resolve_selective_import(
        &mut self,
        selective_idx: usize,
        registry: &std::rc::Rc<std::cell::RefCell<super::module_registry::ModuleRegistry>>,
    ) {
        let Some(module_ident_idx) = self.ast.nodes[selective_idx].next_sibling else {
            return;
        };
        let Some(module_name) = self.ast.node_text(module_ident_idx).map(|s| s.to_string()) else {
            return;
        };
        let module_token = self.ast.nodes[module_ident_idx].token.as_ref().cloned();

        // Collect selector names and their tokens from AST before borrowing registry
        let mut selectors: Vec<(String, Option<crate::lexer::Token>)> = Vec::new();
        let mut current = self.ast.nodes[selective_idx].first_child;
        while let Some(sel_idx) = current {
            if let Some(name) = self.ast.node_text(sel_idx) {
                selectors.push((name.to_string(), self.ast.nodes[sel_idx].token.as_ref().cloned()));
            }
            current = self.ast.nodes[sel_idx].next_sibling;
        }

        let reg = registry.borrow();
        if !reg.module_exists(&module_name) {
            let error_msg = format!("Module '{}' not found", module_name);
            drop(reg);
            if let Some(token) = module_token {
                self.record_error(SemanticError::from_token(error_msg, &token));
            } else {
                self.record_error(SemanticError::new(error_msg, 0, 0));
            }
            return;
        }
        if !self.is_submodule_accessible(&module_name, &reg) {
            let error_msg = format!("Cannot import submodule '{}' from outside its package", module_name);
            drop(reg);
            if let Some(token) = module_token {
                self.record_error(SemanticError::from_token(error_msg, &token));
            } else {
                self.record_error(SemanticError::new(error_msg, 0, 0));
            }
            return;
        }

        let mut symbols_to_add: Vec<Symbol> = Vec::new();
        let mut errors: Vec<(String, Option<crate::lexer::Token>)> = Vec::new();
        for (sel_name, sel_token) in &selectors {
            if let Some(exp) = reg.get_symbol(&module_name, sel_name) {
                symbols_to_add.push(Symbol::new(exp.name.clone(), exp.type_name.clone(), exp.kind));
            } else {
                errors.push((
                    format!("Symbol '{}' not found in module '{}'", sel_name, module_name),
                    sel_token.clone(),
                ));
            }
        }
        drop(reg);

        for sym in symbols_to_add {
            self.scopes.insert(sym);
        }
        for (msg, token) in errors {
            if let Some(t) = token {
                self.record_error(SemanticError::from_token(msg, &t));
            } else {
                self.record_error(SemanticError::new(msg, 0, 0));
            }
        }
    }

    /// Returns the list of exported symbol names collected during analysis.
    ///
    /// Called by `MultiFileAnalyzer` after per-file analysis to build the
    /// `ModuleRegistry`.
    pub fn collect_exported_names(&self) -> Vec<String> {
        self.exported_symbol_names.clone()
    }

    /// Returns true if the named module is accessible from the current file.
    ///
    /// A submodule is only accessible when the importing file is in the same
    /// package batch (`package_modules` contains the module name). In
    /// single-file mode (`package_modules` is None) all imports are allowed.
    fn is_submodule_accessible(
        &self,
        module_name: &str,
        registry: &super::module_registry::ModuleRegistry,
    ) -> bool {
        if !registry.is_submodule(module_name) {
            return true; // Not a submodule — always accessible
        }
        match &self.package_modules {
            None => true, // Single-file mode — no restriction
            Some(pkg) => pkg.contains(module_name),
        }
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

    #[test]
    fn test_export_stmt_duplicate() {
        // Export the same symbol twice — should fail with "exported more than once"
        let source = "x: 42\nexport { x, x }\n";
        let result = analyze_source(source);
        assert!(result.is_err(), "Duplicate export should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Symbol 'x' is exported more than once")),
            "Error should mention duplicate export: {:?}", errors
        );
    }

    #[test]
    fn test_export_stmt_partial_undefined() {
        // Export one defined and one undefined symbol — should fail for the undefined one
        let source = "a: 1\nexport { a, missing }\n";
        let result = analyze_source(source);
        assert!(result.is_err(), "Export of undefined symbol should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Exported symbol 'missing' is not defined")),
            "Error should mention the undefined symbol: {:?}", errors
        );
    }

    #[test]
    fn test_export_collected_names() {
        // After analysis, collect_exported_names() should return the exported names
        let source = "a: 1\nb: 2\nexport { a, b }\n";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let mut analyzer = SemanticAnalyzer::new(ast);

        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
        }

        let names = analyzer.collect_exported_names();
        assert!(names.contains(&"a".to_string()), "collect_exported_names should include 'a'");
        assert!(names.contains(&"b".to_string()), "collect_exported_names should include 'b'");
        assert_eq!(names.len(), 2, "Should have exactly 2 exported names");
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

    // ========== Aliased Import Tests ==========

    #[test]
    fn test_import_aliased_found() {
        use super::super::module_registry::{ModuleExportedSymbol, ModuleRegistry};
        use std::cell::RefCell;
        use std::rc::Rc;

        let source = "import { m: math }\n";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();

        let mut registry = ModuleRegistry::new();
        registry.register_module("math".to_string());
        registry.add_export("math", ModuleExportedSymbol::new("add".to_string(), SymbolKind::Function));
        let registry_rc = Rc::new(RefCell::new(registry));

        let mut analyzer = SemanticAnalyzer::new(ast).with_module_registry(registry_rc);
        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
        }

        // Alias 'm' should be in scope
        let alias_sym = analyzer.scopes.lookup("m");
        assert!(alias_sym.is_some(), "Alias 'm' should be in scope");
        assert_eq!(alias_sym.unwrap().kind, SymbolKind::Module);

        // Original module name 'math' should NOT be in scope
        let math_sym = analyzer.scopes.lookup("math");
        assert!(math_sym.is_none(), "Original module name should NOT be in scope for aliased import");
    }

    #[test]
    fn test_import_aliased_module_not_found() {
        use super::super::module_registry::ModuleRegistry;
        use std::cell::RefCell;
        use std::rc::Rc;

        let source = "import { m: nonexistent }\n";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();

        let registry_rc = Rc::new(RefCell::new(ModuleRegistry::new()));
        let analyzer = SemanticAnalyzer::new(ast).with_module_registry(registry_rc);
        let result = analyzer.analyze();
        assert!(result.is_err(), "Aliased import of unknown module should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Module 'nonexistent' not found")),
            "Error should mention the missing module"
        );
    }

    // ========== Star Import Tests ==========

    #[test]
    fn test_import_star_adds_all_exports() {
        use super::super::module_registry::{ModuleExportedSymbol, ModuleRegistry};
        use std::cell::RefCell;
        use std::rc::Rc;

        let source = "import { *: math }\n";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();

        let mut registry = ModuleRegistry::new();
        registry.register_module("math".to_string());
        registry.add_export("math", ModuleExportedSymbol::new("add".to_string(), SymbolKind::Function));
        registry.add_export("math", ModuleExportedSymbol::new("pi".to_string(), SymbolKind::Variable));
        let registry_rc = Rc::new(RefCell::new(registry));

        let mut analyzer = SemanticAnalyzer::new(ast).with_module_registry(registry_rc);
        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
        }

        let add_sym = analyzer.scopes.lookup("add");
        assert!(add_sym.is_some(), "'add' should be in scope after star import");
        assert_eq!(add_sym.unwrap().kind, SymbolKind::Function);

        let pi_sym = analyzer.scopes.lookup("pi");
        assert!(pi_sym.is_some(), "'pi' should be in scope after star import");
        assert_eq!(pi_sym.unwrap().kind, SymbolKind::Variable);
    }

    #[test]
    fn test_import_star_module_not_found() {
        use super::super::module_registry::ModuleRegistry;
        use std::cell::RefCell;
        use std::rc::Rc;

        let source = "import { *: nonexistent }\n";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();

        let registry_rc = Rc::new(RefCell::new(ModuleRegistry::new()));
        let analyzer = SemanticAnalyzer::new(ast).with_module_registry(registry_rc);
        let result = analyzer.analyze();
        assert!(result.is_err(), "Star import of unknown module should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Module 'nonexistent' not found")),
            "Error should mention the missing module"
        );
    }

    // ========== Selective Import Tests ==========

    #[test]
    fn test_import_selective_found() {
        use super::super::module_registry::{ModuleExportedSymbol, ModuleRegistry};
        use std::cell::RefCell;
        use std::rc::Rc;

        let source = "import { {sin, cos}: math }\n";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();

        let mut registry = ModuleRegistry::new();
        registry.register_module("math".to_string());
        registry.add_export("math", ModuleExportedSymbol::new("sin".to_string(), SymbolKind::Function));
        registry.add_export("math", ModuleExportedSymbol::new("cos".to_string(), SymbolKind::Function));
        registry.add_export("math", ModuleExportedSymbol::new("tan".to_string(), SymbolKind::Function));
        let registry_rc = Rc::new(RefCell::new(registry));

        let mut analyzer = SemanticAnalyzer::new(ast).with_module_registry(registry_rc);
        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
        }

        let sin_sym = analyzer.scopes.lookup("sin");
        assert!(sin_sym.is_some(), "'sin' should be in scope after selective import");
        assert_eq!(sin_sym.unwrap().kind, SymbolKind::Function);

        let cos_sym = analyzer.scopes.lookup("cos");
        assert!(cos_sym.is_some(), "'cos' should be in scope after selective import");

        // 'tan' was not selected — should not be in scope
        let tan_sym = analyzer.scopes.lookup("tan");
        assert!(tan_sym.is_none(), "'tan' should NOT be in scope since it wasn't selected");
    }

    #[test]
    fn test_import_selective_symbol_not_in_module() {
        use super::super::module_registry::{ModuleExportedSymbol, ModuleRegistry};
        use std::cell::RefCell;
        use std::rc::Rc;

        let source = "import { {sin, missing}: math }\n";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();

        let mut registry = ModuleRegistry::new();
        registry.register_module("math".to_string());
        registry.add_export("math", ModuleExportedSymbol::new("sin".to_string(), SymbolKind::Function));
        let registry_rc = Rc::new(RefCell::new(registry));

        let analyzer = SemanticAnalyzer::new(ast).with_module_registry(registry_rc);
        let result = analyzer.analyze();
        assert!(result.is_err(), "Selective import of missing symbol should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Symbol 'missing' not found in module 'math'")),
            "Error should mention the missing symbol: {:?}", errors
        );
    }

    #[test]
    fn test_import_selective_module_not_found() {
        use super::super::module_registry::ModuleRegistry;
        use std::cell::RefCell;
        use std::rc::Rc;

        let source = "import { {sin}: nonexistent }\n";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();

        let registry_rc = Rc::new(RefCell::new(ModuleRegistry::new()));
        let analyzer = SemanticAnalyzer::new(ast).with_module_registry(registry_rc);
        let result = analyzer.analyze();
        assert!(result.is_err(), "Selective import from unknown module should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Module 'nonexistent' not found")),
            "Error should mention the missing module"
        );
    }

    // ========== Submodule Visibility Tests — Full Import ==========

    fn make_registry_with_submodule(name: &str) -> std::rc::Rc<std::cell::RefCell<super::super::module_registry::ModuleRegistry>> {
        use super::super::module_registry::ModuleRegistry;
        use std::cell::RefCell;
        use std::rc::Rc;
        let mut registry = ModuleRegistry::new();
        registry.register_submodule(name.to_string());
        Rc::new(RefCell::new(registry))
    }

    fn make_package_modules(names: &[&str]) -> std::collections::HashSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_submodule_import_allowed_with_package_context() {
        // registry has submodule 'utils', package_modules contains 'utils' → no error
        let source = "import { utils }\n";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();

        let registry_rc = make_registry_with_submodule("utils");
        let pkg = make_package_modules(&["utils"]);

        let analyzer = SemanticAnalyzer::new(ast)
            .with_module_registry(registry_rc)
            .with_package_modules(pkg);
        let result = analyzer.analyze();
        assert!(result.is_ok(), "Submodule import within package should succeed: {:?}", result);
    }

    #[test]
    fn test_submodule_import_denied_outside_package() {
        // registry has submodule 'utils', package_modules does NOT contain 'utils' → error
        let source = "import { utils }\n";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();

        let registry_rc = make_registry_with_submodule("utils");
        let pkg = make_package_modules(&[]); // empty — 'utils' not in this batch

        let analyzer = SemanticAnalyzer::new(ast)
            .with_module_registry(registry_rc)
            .with_package_modules(pkg);
        let result = analyzer.analyze();
        assert!(result.is_err(), "Submodule import from outside package should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Cannot import submodule 'utils' from outside its package")),
            "Error should mention submodule access restriction: {:?}", errors
        );
    }

    #[test]
    fn test_submodule_import_allowed_no_package_context() {
        // No package_modules set (single-file mode) → always allowed
        let source = "import { utils }\n";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();

        let registry_rc = make_registry_with_submodule("utils");
        // No with_package_modules call — single-file mode

        let analyzer = SemanticAnalyzer::new(ast)
            .with_module_registry(registry_rc);
        let result = analyzer.analyze();
        assert!(result.is_ok(), "Submodule import in single-file mode should be allowed: {:?}", result);
    }

    // ========== Submodule Visibility Tests — Aliased Import ==========

    #[test]
    fn test_submodule_aliased_import_allowed_with_package_context() {
        let source = "import { u: utils }\n";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();

        let registry_rc = make_registry_with_submodule("utils");
        let pkg = make_package_modules(&["utils"]);

        let analyzer = SemanticAnalyzer::new(ast)
            .with_module_registry(registry_rc)
            .with_package_modules(pkg);
        let result = analyzer.analyze();
        assert!(result.is_ok(), "Aliased submodule import within package should succeed: {:?}", result);
    }

    #[test]
    fn test_submodule_aliased_import_denied_outside_package() {
        let source = "import { u: utils }\n";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();

        let registry_rc = make_registry_with_submodule("utils");
        let pkg = make_package_modules(&[]);

        let analyzer = SemanticAnalyzer::new(ast)
            .with_module_registry(registry_rc)
            .with_package_modules(pkg);
        let result = analyzer.analyze();
        assert!(result.is_err(), "Aliased submodule import from outside package should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Cannot import submodule 'utils' from outside its package")),
            "Error should mention submodule restriction: {:?}", errors
        );
    }

    #[test]
    fn test_submodule_aliased_import_allowed_no_package_context() {
        let source = "import { u: utils }\n";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();

        let registry_rc = make_registry_with_submodule("utils");

        let analyzer = SemanticAnalyzer::new(ast).with_module_registry(registry_rc);
        let result = analyzer.analyze();
        assert!(result.is_ok(), "Aliased submodule import in single-file mode should be allowed: {:?}", result);
    }

    // ========== Submodule Visibility Tests — Star Import ==========

    #[test]
    fn test_submodule_star_import_allowed_with_package_context() {
        let source = "import { *: utils }\n";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();

        let registry_rc = make_registry_with_submodule("utils");
        let pkg = make_package_modules(&["utils"]);

        let analyzer = SemanticAnalyzer::new(ast)
            .with_module_registry(registry_rc)
            .with_package_modules(pkg);
        let result = analyzer.analyze();
        assert!(result.is_ok(), "Star submodule import within package should succeed: {:?}", result);
    }

    #[test]
    fn test_submodule_star_import_denied_outside_package() {
        let source = "import { *: utils }\n";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();

        let registry_rc = make_registry_with_submodule("utils");
        let pkg = make_package_modules(&[]);

        let analyzer = SemanticAnalyzer::new(ast)
            .with_module_registry(registry_rc)
            .with_package_modules(pkg);
        let result = analyzer.analyze();
        assert!(result.is_err(), "Star submodule import from outside package should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Cannot import submodule 'utils' from outside its package")),
            "Error should mention submodule restriction: {:?}", errors
        );
    }

    #[test]
    fn test_submodule_star_import_allowed_no_package_context() {
        let source = "import { *: utils }\n";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();

        let registry_rc = make_registry_with_submodule("utils");

        let analyzer = SemanticAnalyzer::new(ast).with_module_registry(registry_rc);
        let result = analyzer.analyze();
        assert!(result.is_ok(), "Star submodule import in single-file mode should be allowed: {:?}", result);
    }

    // ========== Submodule Visibility Tests — Selective Import ==========

    #[test]
    fn test_submodule_selective_import_allowed_with_package_context() {
        use super::super::module_registry::{ModuleExportedSymbol, ModuleRegistry};
        use std::cell::RefCell;
        use std::rc::Rc;

        let source = "import { {helper}: utils }\n";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();

        let mut registry = ModuleRegistry::new();
        registry.register_submodule("utils".to_string());
        registry.add_export("utils", ModuleExportedSymbol::new("helper".to_string(), SymbolKind::Function));
        let registry_rc = Rc::new(RefCell::new(registry));
        let pkg = make_package_modules(&["utils"]);

        let analyzer = SemanticAnalyzer::new(ast)
            .with_module_registry(registry_rc)
            .with_package_modules(pkg);
        let result = analyzer.analyze();
        assert!(result.is_ok(), "Selective submodule import within package should succeed: {:?}", result);
    }

    #[test]
    fn test_submodule_selective_import_denied_outside_package() {
        use super::super::module_registry::{ModuleExportedSymbol, ModuleRegistry};
        use std::cell::RefCell;
        use std::rc::Rc;

        let source = "import { {helper}: utils }\n";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();

        let mut registry = ModuleRegistry::new();
        registry.register_submodule("utils".to_string());
        registry.add_export("utils", ModuleExportedSymbol::new("helper".to_string(), SymbolKind::Function));
        let registry_rc = Rc::new(RefCell::new(registry));
        let pkg = make_package_modules(&[]);

        let analyzer = SemanticAnalyzer::new(ast)
            .with_module_registry(registry_rc)
            .with_package_modules(pkg);
        let result = analyzer.analyze();
        assert!(result.is_err(), "Selective submodule import from outside package should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Cannot import submodule 'utils' from outside its package")),
            "Error should mention submodule restriction: {:?}", errors
        );
    }

    #[test]
    fn test_submodule_selective_import_allowed_no_package_context() {
        use super::super::module_registry::{ModuleExportedSymbol, ModuleRegistry};
        use std::cell::RefCell;
        use std::rc::Rc;

        let source = "import { {helper}: utils }\n";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();

        let mut registry = ModuleRegistry::new();
        registry.register_submodule("utils".to_string());
        registry.add_export("utils", ModuleExportedSymbol::new("helper".to_string(), SymbolKind::Function));
        let registry_rc = Rc::new(RefCell::new(registry));

        let analyzer = SemanticAnalyzer::new(ast).with_module_registry(registry_rc);
        let result = analyzer.analyze();
        assert!(result.is_ok(), "Selective submodule import in single-file mode should be allowed: {:?}", result);
    }
}
