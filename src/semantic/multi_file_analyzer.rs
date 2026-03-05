// Multi-file semantic analysis pipeline.
//
// Implements a two-pass approach:
//   Pass 1: Lightweight AST scan to collect module names and exported symbols
//           per file, used to build a shared ModuleRegistry.
//   Pass 2: Full SemanticAnalyzer run per file, using the shared registry
//           for import resolution.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::ast::{Ast, NodeType};
use crate::limits::CompilerLimits;

use super::{module_registry::{ModuleExportedSymbol, ModuleRegistry}, SemanticAnalyzer, SemanticError, SymbolKind};

/// A source file to be analyzed
pub struct SourceFile {
    pub name: String,
    pub source: String,
}

/// The result of analyzing a single file
pub struct FileAnalysisResult {
    /// Semantic errors detected in this file
    pub errors: Vec<SemanticError>,
    /// The module name declared in this file (if any)
    pub module_name: Option<String>,
}

/// Two-pass multi-file semantic analyzer.
///
/// Usage:
/// ```ignore
/// let analyzer = MultiFileAnalyzer::new(vec![
///     SourceFile { name: "math.suru".into(), source: math_src },
///     SourceFile { name: "main.suru".into(), source: main_src },
/// ]);
/// let results = analyzer.analyze();
/// ```
pub struct MultiFileAnalyzer {
    sources: Vec<SourceFile>,
    limits: CompilerLimits,
}

impl MultiFileAnalyzer {
    /// Creates a new multi-file analyzer with default compiler limits.
    pub fn new(sources: Vec<SourceFile>) -> Self {
        MultiFileAnalyzer { sources, limits: CompilerLimits::default() }
    }

    /// Runs the two-pass analysis and returns per-file results.
    ///
    /// # Algorithm
    ///
    /// 1. Parse all source files; collect parse errors per file.
    /// 2. First pass (lightweight): scan each successfully-parsed AST for
    ///    `ModuleDecl` and `Export` node names → build `ModuleRegistry`.
    /// 3. Second pass: run full `SemanticAnalyzer` on each file, sharing
    ///    the registry so imports can be resolved.
    pub fn analyze(&self) -> HashMap<String, FileAnalysisResult> {
        // ── Step 1: parse all files ──────────────────────────────────────────
        let mut parsed: Vec<(String, Result<Ast, String>)> = Vec::new();
        for sf in &self.sources {
            let result = self.parse_source(&sf.source);
            parsed.push((sf.name.clone(), result));
        }

        // ── Step 2: first pass — collect module info ─────────────────────────
        let registry = Rc::new(RefCell::new(ModuleRegistry::new()));
        let mut file_module_names: HashMap<String, Option<String>> = HashMap::new();

        // Sub-step A: collect (name, module_name, is_submodule, exports) without registering
        let mut collected: Vec<(String, Option<String>, bool, Vec<String>)> = Vec::new();
        for (name, result) in &parsed {
            if let Ok(ast) = result {
                let (module_name, export_names, is_submodule) = extract_module_info(ast);
                file_module_names.insert(name.clone(), module_name.clone());
                collected.push((name.clone(), module_name, is_submodule, export_names));
            } else {
                file_module_names.insert(name.clone(), None);
                collected.push((name.clone(), None, false, Vec::new()));
            }
        }

        // Sub-step B: find the single main (non-submodule) module name in this batch,
        // then register all modules with proper parent links.
        let main_module_name: Option<String> = collected
            .iter()
            .find(|(_, _, is_sub, _)| !is_sub)
            .and_then(|(_, mod_name, _, _)| mod_name.clone());

        for (_, module_name, is_submodule, export_names) in &collected {
            if let Some(mod_name) = module_name {
                let mut reg = registry.borrow_mut();
                if *is_submodule {
                    if let Some(ref parent) = main_module_name {
                        reg.register_submodule_with_parent(mod_name.clone(), parent.clone());
                    } else {
                        reg.register_submodule(mod_name.clone());
                    }
                } else {
                    reg.register_module(mod_name.clone());
                }
                for export_name in export_names {
                    reg.add_export(
                        mod_name,
                        ModuleExportedSymbol::new(export_name.clone(), SymbolKind::Variable),
                    );
                }
            }
        }

        // Collect all module names in this batch for submodule visibility enforcement
        let package_modules: HashSet<String> = file_module_names
            .values()
            .filter_map(|opt| opt.as_ref())
            .cloned()
            .collect();

        // ── Step 3: second pass — full semantic analysis ─────────────────────
        let mut results: HashMap<String, FileAnalysisResult> = HashMap::new();

        for (name, parse_result) in parsed {
            match parse_result {
                Err(parse_error) => {
                    // File failed to parse — report as a semantic error
                    results.insert(name.clone(), FileAnalysisResult {
                        errors: vec![SemanticError::new(
                            format!("Parse error: {}", parse_error),
                            0,
                            0,
                        )],
                        module_name: file_module_names.get(&name).cloned().flatten(),
                    });
                }
                Ok(ast) => {
                    let module_name = file_module_names.get(&name).cloned().flatten();
                    let analyzer = SemanticAnalyzer::new(ast)
                        .with_module_registry(registry.clone())
                        .with_package_modules(package_modules.clone());
                    let errors = match analyzer.analyze() {
                        Ok(_) => Vec::new(),
                        Err(errs) => errs,
                    };
                    results.insert(name, FileAnalysisResult { errors, module_name });
                }
            }
        }

        results
    }

    /// Parses a source string into an AST, returning an error string on failure.
    fn parse_source(&self, source: &str) -> Result<Ast, String> {
        let tokens = crate::lexer::lex(source, &self.limits)
            .map_err(|e| e.to_string())?;
        crate::parser::parse(tokens, &self.limits)
            .map_err(|e| e.to_string())
    }
}

/// Lightweight first-pass scan: extracts module name, exported symbol names,
/// and whether the module is a submodule directly from the AST without running
/// full type-checking.
///
/// Returns `(module_name, export_names, is_submodule)`.
pub fn extract_module_info(ast: &Ast) -> (Option<String>, Vec<String>, bool) {
    let Some(root_idx) = ast.root else {
        return (None, Vec::new(), false);
    };

    let mut module_name: Option<String> = None;
    let mut export_names: Vec<String> = Vec::new();
    let mut is_submodule = false;

    // Walk direct children of Program
    let mut current = ast.nodes[root_idx].first_child;
    while let Some(child_idx) = current {
        match ast.nodes[child_idx].node_type {
            NodeType::ModuleDecl => {
                // ModuleDecl → ModulePath (terminal)
                if let Some(path_idx) = ast.nodes[child_idx].first_child {
                    if let Some(text) = ast.node_text(path_idx) {
                        let text = text.to_string();
                        // Strip leading dot for submodules
                        if text.starts_with('.') {
                            is_submodule = true;
                            module_name = Some(text[1..].to_string());
                        } else {
                            module_name = Some(text);
                        }
                    }
                }
            }
            NodeType::Export => {
                // Export → ExportList → Identifier*
                if let Some(list_idx) = ast.nodes[child_idx].first_child {
                    let mut ident = ast.nodes[list_idx].first_child;
                    while let Some(ident_idx) = ident {
                        if let Some(name) = ast.node_text(ident_idx) {
                            export_names.push(name.to_string());
                        }
                        ident = ast.nodes[ident_idx].next_sibling;
                    }
                }
            }
            _ => {}
        }
        current = ast.nodes[child_idx].next_sibling;
    }

    (module_name, export_names, is_submodule)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_file(name: &str, source: &str) -> SourceFile {
        SourceFile { name: name.to_string(), source: source.to_string() }
    }

    // ── extract_module_info tests ────────────────────────────────────────────

    #[test]
    fn test_extract_module_info_no_module() {
        let limits = CompilerLimits::default();
        let tokens = crate::lexer::lex("x: 42\n", &limits).unwrap();
        let ast = crate::parser::parse(tokens, &limits).unwrap();
        let (module_name, exports, is_submodule) = extract_module_info(&ast);
        assert!(module_name.is_none());
        assert!(exports.is_empty());
        assert!(!is_submodule);
    }

    #[test]
    fn test_extract_module_info_with_module() {
        let limits = CompilerLimits::default();
        let tokens = crate::lexer::lex("module math\n", &limits).unwrap();
        let ast = crate::parser::parse(tokens, &limits).unwrap();
        let (module_name, _, is_submodule) = extract_module_info(&ast);
        assert_eq!(module_name, Some("math".to_string()));
        assert!(!is_submodule);
    }

    #[test]
    fn test_extract_module_info_with_exports() {
        let source = "module math\nadd: (x Number, y Number) Number { return x }\nexport { add }\n";
        let limits = CompilerLimits::default();
        let tokens = crate::lexer::lex(source, &limits).unwrap();
        let ast = crate::parser::parse(tokens, &limits).unwrap();
        let (module_name, exports, is_submodule) = extract_module_info(&ast);
        assert_eq!(module_name, Some("math".to_string()));
        assert!(exports.contains(&"add".to_string()));
        assert!(!is_submodule);
    }

    #[test]
    fn test_extract_module_info_submodule() {
        let limits = CompilerLimits::default();
        let tokens = crate::lexer::lex("module .utils\n", &limits).unwrap();
        let ast = crate::parser::parse(tokens, &limits).unwrap();
        let (module_name, _, is_submodule) = extract_module_info(&ast);
        assert_eq!(module_name, Some("utils".to_string()));
        assert!(is_submodule);
    }

    // ── MultiFileAnalyzer integration tests ──────────────────────────────────

    #[test]
    fn test_single_file_no_module() {
        let analyzer = MultiFileAnalyzer::new(vec![make_file("main.suru", "x: 42\n")]);
        let results = analyzer.analyze();
        let r = results.get("main.suru").unwrap();
        assert!(r.errors.is_empty(), "Simple file should have no errors: {:?}", r.errors);
        assert!(r.module_name.is_none());
    }

    #[test]
    fn test_single_file_with_module() {
        let analyzer = MultiFileAnalyzer::new(vec![make_file(
            "math.suru",
            "module math\nx: 1\n",
        )]);
        let results = analyzer.analyze();
        let r = results.get("math.suru").unwrap();
        assert!(r.errors.is_empty(), "Module file should have no errors: {:?}", r.errors);
        assert_eq!(r.module_name, Some("math".to_string()));
    }

    #[test]
    fn test_two_files_basic() {
        let math_src = "module math\nadd: (x Number, y Number) Number { return x }\n";
        let main_src = "x: 42\n";

        let analyzer = MultiFileAnalyzer::new(vec![
            make_file("math.suru", math_src),
            make_file("main.suru", main_src),
        ]);
        let results = analyzer.analyze();

        assert!(results.contains_key("math.suru"));
        assert!(results.contains_key("main.suru"));
        assert!(results["math.suru"].errors.is_empty(), "math.suru should have no errors: {:?}", results["math.suru"].errors);
        assert!(results["main.suru"].errors.is_empty(), "main.suru should have no errors: {:?}", results["main.suru"].errors);
    }

    #[test]
    fn test_two_files_import_resolution() {
        let math_src = "module math\nadd: (x Number, y Number) Number { return x }\nexport { add }\n";
        let main_src = "import { math }\n";

        let analyzer = MultiFileAnalyzer::new(vec![
            make_file("math.suru", math_src),
            make_file("main.suru", main_src),
        ]);
        let results = analyzer.analyze();

        let main_result = &results["main.suru"];
        assert!(
            main_result.errors.is_empty(),
            "Importing a known module should produce no errors: {:?}",
            main_result.errors
        );
    }

    #[test]
    fn test_two_files_import_not_found() {
        let main_src = "import { nonexistent }\n";

        let analyzer = MultiFileAnalyzer::new(vec![make_file("main.suru", main_src)]);
        let results = analyzer.analyze();

        let main_result = &results["main.suru"];
        assert!(
            !main_result.errors.is_empty(),
            "Importing unknown module should produce errors"
        );
        assert!(
            main_result.errors.iter().any(|e| e.message.contains("Module 'nonexistent' not found")),
            "Error should mention the missing module"
        );
    }

    // ── Submodule visibility tests ────────────────────────────────────────────

    #[test]
    fn test_submodule_import_from_main_module() {
        // Both files in the same batch — main module can import submodule
        let main_src = "module Calculator\nimport { utils }\n";
        let utils_src = "module .utils\n";

        let analyzer = MultiFileAnalyzer::new(vec![
            make_file("calc.suru", main_src),
            make_file("utils.suru", utils_src),
        ]);
        let results = analyzer.analyze();

        let calc_result = &results["calc.suru"];
        assert!(
            calc_result.errors.is_empty(),
            "Main module importing sibling submodule should succeed: {:?}",
            calc_result.errors
        );
    }

    #[test]
    fn test_submodule_import_from_sibling_submodule() {
        // Both submodules in the same batch — sibling can import sibling
        let ops_src = "module .operations\nimport { utils }\n";
        let utils_src = "module .utils\n";

        let analyzer = MultiFileAnalyzer::new(vec![
            make_file("operations.suru", ops_src),
            make_file("utils.suru", utils_src),
        ]);
        let results = analyzer.analyze();

        let ops_result = &results["operations.suru"];
        assert!(
            ops_result.errors.is_empty(),
            "Sibling submodule import should succeed: {:?}",
            ops_result.errors
        );
    }

    #[test]
    fn test_submodule_registered_with_parent() {
        // After batch analysis, qualified import Calculator.utils should work,
        // which proves the parent link was registered in the registry.
        let calc_src = "module Calculator\n";
        let utils_src = "module .utils\n";
        // A third file in the same batch imports via qualified path
        let main_src = "import { Calculator.utils }\n";

        let analyzer = MultiFileAnalyzer::new(vec![
            make_file("calc.suru", calc_src),
            make_file("utils.suru", utils_src),
            make_file("main.suru", main_src),
        ]);
        let results = analyzer.analyze();

        // If parent link was registered, the qualified import resolves to "utils" and succeeds
        let main_result = &results["main.suru"];
        assert!(
            main_result.errors.is_empty(),
            "Qualified import should succeed when parent link is registered: {:?}",
            main_result.errors
        );
    }

    #[test]
    fn test_qualified_submodule_import() {
        // import { Calculator.utils } should succeed when both files are in the same batch
        let calc_src = "module Calculator\n";
        let utils_src = "module .utils\n";
        let main_src = "import { Calculator.utils }\n";

        let analyzer = MultiFileAnalyzer::new(vec![
            make_file("calc.suru", calc_src),
            make_file("utils.suru", utils_src),
            make_file("main.suru", main_src),
        ]);
        let results = analyzer.analyze();

        assert!(results["calc.suru"].errors.is_empty(), "calc.suru errors: {:?}", results["calc.suru"].errors);
        assert!(results["utils.suru"].errors.is_empty(), "utils.suru errors: {:?}", results["utils.suru"].errors);
        let main_result = &results["main.suru"];
        assert!(
            main_result.errors.is_empty(),
            "import {{ Calculator.utils }} should succeed: {:?}",
            main_result.errors
        );
    }

    #[test]
    fn test_qualified_submodule_wrong_parent_fails() {
        // import { Wrong.utils } should fail even when .utils exists (wrong parent)
        let utils_src = "module .utils\n";
        let main_src = "import { Wrong.utils }\n";

        let analyzer = MultiFileAnalyzer::new(vec![
            make_file("utils.suru", utils_src),
            make_file("main.suru", main_src),
        ]);
        let results = analyzer.analyze();

        let main_result = &results["main.suru"];
        assert!(
            !main_result.errors.is_empty(),
            "import {{ Wrong.utils }} should fail when parent is incorrect"
        );
        assert!(
            main_result.errors.iter().any(|e| e.message.contains("not found")),
            "Error should mention not found: {:?}",
            main_result.errors
        );
    }

    #[test]
    fn test_submodule_not_importable_from_outside() {
        use super::super::module_registry::ModuleRegistry;
        use std::collections::HashSet;
        use std::rc::Rc;

        // Simulate batch A: register 'utils' as a submodule
        let mut registry = ModuleRegistry::new();
        registry.register_submodule("utils".to_string());
        let registry_rc = Rc::new(RefCell::new(registry));

        // Batch B: new analyzer with the registry but empty package_modules
        // (simulates an external file not in the same batch as utils)
        let limits = CompilerLimits::default();
        let tokens = crate::lexer::lex("import { utils }\n", &limits).unwrap();
        let ast = crate::parser::parse(tokens, &limits).unwrap();

        let analyzer = SemanticAnalyzer::new(ast)
            .with_module_registry(registry_rc)
            .with_package_modules(HashSet::new()); // empty — 'utils' not in this batch

        let result = analyzer.analyze();
        assert!(result.is_err(), "External file should not be able to import submodule");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Cannot import submodule 'utils' from outside its package")),
            "Error should mention submodule access restriction: {:?}", errors
        );
    }
}
