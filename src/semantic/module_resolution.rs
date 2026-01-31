// Module declaration resolution for Phase 6.1
//
// This module implements:
// - Module declaration registration
// - Main module handling (module Name)
// - Submodule handling (module .name)
// - Module symbol table creation

use super::{SemanticAnalyzer, SemanticError, Symbol, SymbolKind, ScopeKind};

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
    fn analyze_and_get_module(source: &str) -> (Option<String>, bool, Result<crate::ast::Ast, Vec<SemanticError>>) {
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
        assert!(errors[0].message.contains("Only one module declaration allowed"));
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
        assert!(errors[0].message.contains("Only one module declaration allowed"));
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
        assert!(symbol.is_some(), "Variable should be visible in module scope");
    }
}
