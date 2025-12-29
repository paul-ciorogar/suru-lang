use super::{ParseError, Parser};
use crate::ast::{AstNode, NodeType};
use crate::lexer::{Token, TokenKind};

impl<'a> Parser<'a> {
    /// Parse module declaration: module Name or module .name
    pub(super) fn parse_module_decl(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Consume 'module' keyword
        self.consume(TokenKind::Module, "'module'")?;
        self.skip_newlines();

        // Parse module path
        let path_string = self.parse_module_path()?;

        // Intern the path string
        let string_id = self.ast.string_storage.intern(&path_string);
        let token = Token {
            kind: TokenKind::Identifier,
            line: self.current_token().line,
            column: self.current_token().column,
            string_id: Some(string_id),
        };

        // Create ModuleDecl node
        let module_decl_node = AstNode::new(NodeType::ModuleDecl);
        let module_decl_idx = self.ast.add_node(module_decl_node);

        // Create ModulePath node
        let path_node = AstNode::new_terminal(NodeType::ModulePath, token);
        let path_idx = self.ast.add_node(path_node);
        self.ast.add_child(module_decl_idx, path_idx);

        Ok(module_decl_idx)
    }

    /// Parse export statement: export { ... }
    pub(super) fn parse_export_stmt(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Consume 'export' keyword
        self.consume(TokenKind::Export, "'export'")?;
        self.skip_newlines();

        // Consume '{'
        self.consume(TokenKind::LBrace, "'{'")?;

        // Create Export and ExportList nodes
        let export_node = AstNode::new(NodeType::Export);
        let export_idx = self.ast.add_node(export_node);

        let export_list_node = AstNode::new(NodeType::ExportList);
        let export_list_idx = self.ast.add_node(export_list_node);
        self.ast.add_child(export_idx, export_list_idx);

        // Parse export list
        loop {
            self.skip_newlines();

            // Check for closing brace
            if self.peek_kind_is(TokenKind::RBrace) {
                self.advance();
                break;
            }

            // Parse identifier
            if self.peek_kind() != TokenKind::Identifier {
                return Err(self.new_unexpected_token("identifier"));
            }

            let ident_node = AstNode::new_terminal(NodeType::Identifier, self.clone_current_token());
            let ident_idx = self.ast.add_node(ident_node);
            self.ast.add_child(export_list_idx, ident_idx);
            self.advance();

            // Check what comes next (without skipping newlines first)
            match self.peek_kind() {
                TokenKind::Newline => {
                    // Newline separator - skip and continue
                    self.skip_newlines();
                }
                TokenKind::Comma => {
                    // Comma separator
                    self.advance();
                    self.skip_newlines();
                    // Allow trailing comma
                    if self.peek_kind_is(TokenKind::RBrace) {
                        self.advance();
                        break;
                    }
                }
                TokenKind::RBrace => {
                    // End of list
                    self.advance();
                    break;
                }
                _ => {
                    // Error - need comma or newline to separate items
                    return Err(self.new_unexpected_token("',' or '}'"));
                }
            }
        }

        Ok(export_idx)
    }

    /// Parse import statement: import { ... }
    pub(super) fn parse_import_stmt(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        // Consume 'import' keyword
        self.consume(TokenKind::Import, "'import'")?;
        self.skip_newlines();

        // Consume '{'
        self.consume(TokenKind::LBrace, "'{'")?;

        // Create Import and ImportList nodes
        let import_node = AstNode::new(NodeType::Import);
        let import_idx = self.ast.add_node(import_node);

        let import_list_node = AstNode::new(NodeType::ImportList);
        let import_list_idx = self.ast.add_node(import_list_node);
        self.ast.add_child(import_idx, import_list_idx);

        // Parse import items
        loop {
            self.skip_newlines();

            // Check for closing brace
            if self.peek_kind_is(TokenKind::RBrace) {
                self.advance();
                break;
            }

            // Parse import item
            let item_idx = self.parse_import_item(depth + 1)?;
            self.ast.add_child(import_list_idx, item_idx);

            // Check what comes next (without skipping newlines first)
            match self.peek_kind() {
                TokenKind::Newline => {
                    // Newline separator - skip and continue
                    self.skip_newlines();
                }
                TokenKind::Comma => {
                    // Comma separator
                    self.advance();
                    self.skip_newlines();
                    // Allow trailing comma
                    if self.peek_kind_is(TokenKind::RBrace) {
                        self.advance();
                        break;
                    }
                }
                TokenKind::RBrace => {
                    // End of list
                    self.advance();
                    break;
                }
                _ => {
                    // Error - need comma or newline to separate items
                    return Err(self.new_unexpected_token("',' or '}'"));
                }
            }
        }

        Ok(import_idx)
    }

    /// Parse single import item (full, aliased, selective, or star)
    fn parse_import_item(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;
        self.skip_newlines();

        let item_node = AstNode::new(NodeType::ImportItem);
        let item_idx = self.ast.add_node(item_node);

        match self.peek_kind() {
            TokenKind::LBrace => {
                // Selective import: {sin, cos}: math
                let selective_idx = self.parse_import_selective(depth + 1)?;
                self.ast.add_child(item_idx, selective_idx);

                self.skip_newlines();
                self.consume(TokenKind::Colon, "':'")?;

                self.skip_newlines();
                let module_idx = self.parse_identifier_or_path()?;
                self.ast.add_child(item_idx, module_idx);
            }

            TokenKind::Star => {
                // Star import: *: math
                let star_node =
                    AstNode::new_terminal(NodeType::Identifier, self.clone_current_token());
                let star_idx = self.ast.add_node(star_node);
                self.ast.add_child(item_idx, star_idx);
                self.advance();

                self.skip_newlines();
                self.consume(TokenKind::Colon, "':'")?;

                self.skip_newlines();
                let module_idx = self.parse_identifier_or_path()?;
                self.ast.add_child(item_idx, module_idx);
            }

            TokenKind::Identifier => {
                // Full or aliased import
                // First, check if this is an alias by looking for a simple identifier followed by colon
                let first_token = self.clone_current_token();

                // Lookahead: if next token (after skipping newlines) is colon, this is an alias
                // Save position to backtrack if needed
                let saved_pos = self.current;
                self.advance();

                // Skip newlines to check for colon
                let mut had_newlines = false;
                while self.peek_kind_is(TokenKind::Newline) {
                    had_newlines = true;
                    self.advance();
                }

                if self.peek_kind_is(TokenKind::Colon) && !had_newlines {
                    // Aliased: alias: math.module
                    let alias_node = AstNode::new_terminal(NodeType::ImportAlias, first_token);
                    let alias_idx = self.ast.add_node(alias_node);
                    self.ast.add_child(item_idx, alias_idx);

                    self.advance(); // consume ':'
                    self.skip_newlines();

                    let module_idx = self.parse_identifier_or_path()?;
                    self.ast.add_child(item_idx, module_idx);
                } else {
                    // Full import: math or math.geometry
                    // Backtrack and parse full identifier or path
                    self.current = saved_pos;
                    let module_idx = self.parse_identifier_or_path()?;
                    self.ast.add_child(item_idx, module_idx);
                }
            }

            _ => return Err(self.new_unexpected_token("import item")),
        }

        Ok(item_idx)
    }

    /// Parse selective import list: {sin, cos, pi}
    fn parse_import_selective(&mut self, depth: usize) -> Result<usize, ParseError> {
        self.check_depth(depth)?;

        self.consume(TokenKind::LBrace, "'{'")?;

        let selective_node = AstNode::new(NodeType::ImportSelective);
        let selective_idx = self.ast.add_node(selective_node);

        loop {
            self.skip_newlines();

            // Check for closing brace
            if self.peek_kind_is(TokenKind::RBrace) {
                self.advance();
                break;
            }

            // Parse selector (identifier)
            if self.peek_kind() != TokenKind::Identifier {
                return Err(self.new_unexpected_token("identifier"));
            }

            let selector_node =
                AstNode::new_terminal(NodeType::ImportSelector, self.clone_current_token());
            let selector_idx = self.ast.add_node(selector_node);
            self.ast.add_child(selective_idx, selector_idx);
            self.advance();

            self.skip_newlines();

            // Check for comma or closing brace
            match self.peek_kind() {
                TokenKind::RBrace => {
                    self.advance();
                    break;
                }
                TokenKind::Comma => {
                    self.advance();
                    // Allow trailing comma
                    self.skip_newlines();
                    if self.peek_kind_is(TokenKind::RBrace) {
                        self.advance();
                        break;
                    }
                }
                _ => return Err(self.new_unexpected_token("',' or '}'")),
            }
        }

        Ok(selective_idx)
    }

    /// Helper: Parse module path (handles dots and submodules)
    fn parse_module_path(&mut self) -> Result<String, ParseError> {
        let mut parts = Vec::new();

        // Check for leading dot (submodule)
        if self.peek_kind_is(TokenKind::Dot) {
            parts.push(".".to_string());
            self.advance();
        }

        // Parse first identifier
        if self.peek_kind() != TokenKind::Identifier {
            return Err(self.new_unexpected_token("module name"));
        }
        let name = self
            .current_token()
            .text(&self.ast.string_storage)
            .ok_or_else(|| self.new_unexpected_token("module name"))?
            .to_string();
        parts.push(name);
        let is_submodule = parts[0] == ".";
        self.advance();

        // Parse additional dot-separated segments (for paths like math.geometry)
        // Don't allow additional dots after submodule prefix
        while self.peek_kind_is(TokenKind::Dot) && !is_submodule {
            self.advance(); // consume dot
            if self.peek_kind() != TokenKind::Identifier {
                return Err(self.new_unexpected_token("identifier after '.'"));
            }
            parts.push(".".to_string());
            let name = self
                .current_token()
                .text(&self.ast.string_storage)
                .ok_or_else(|| self.new_unexpected_token("identifier"))?
                .to_string();
            parts.push(name);
            self.advance();
        }

        Ok(parts.join(""))
    }

    /// Helper: Parse identifier or dotted path (for module references in imports)
    fn parse_identifier_or_path(&mut self) -> Result<usize, ParseError> {
        if self.peek_kind() != TokenKind::Identifier {
            return Err(self.new_unexpected_token("identifier"));
        }

        let mut parts = Vec::new();

        // Parse first identifier
        let name = self
            .current_token()
            .text(&self.ast.string_storage)
            .ok_or_else(|| self.new_unexpected_token("identifier"))?
            .to_string();
        parts.push(name);
        let first_token = self.clone_current_token();
        self.advance();

        // Parse additional dot-separated segments
        while self.peek_kind_is(TokenKind::Dot) {
            // Lookahead to see if there's an identifier after the dot
            if self.peek_next_kind(1) != TokenKind::Identifier {
                break;
            }
            self.advance(); // consume dot
            parts.push(".".to_string());
            let name = self
                .current_token()
                .text(&self.ast.string_storage)
                .ok_or_else(|| self.new_unexpected_token("identifier"))?
                .to_string();
            parts.push(name);
            self.advance();
        }

        // If we have a dotted path, intern it
        if parts.len() > 1 {
            let path_string = parts.join("");
            let string_id = self.ast.string_storage.intern(&path_string);
            let token = Token {
                kind: TokenKind::Identifier,
                line: first_token.line,
                column: first_token.column,
                string_id: Some(string_id),
            };
            let ident_node = AstNode::new_terminal(NodeType::Identifier, token);
            let ident_idx = self.ast.add_node(ident_node);
            Ok(ident_idx)
        } else {
            // Simple identifier
            let ident_node = AstNode::new_terminal(NodeType::Identifier, first_token);
            let ident_idx = self.ast.add_node(ident_node);
            Ok(ident_idx)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::lexer::lex;
    use crate::limits::CompilerLimits;
    use crate::parser;

    fn to_ast_string(source: &str) -> String {
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parser::parse(tokens, &limits).unwrap();
        ast.to_string()
    }

    fn to_ast(source: &str) -> Result<crate::ast::Ast, crate::parser::ParseError> {
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        parser::parse(tokens, &limits)
    }

    // ========== Module Declaration Tests ==========

    #[test]
    fn test_module_simple() {
        assert_eq!(
            to_ast_string("module Calculator\n"),
            "Program\n  ModuleDecl\n    ModulePath 'Calculator'\n"
        );
    }

    #[test]
    fn test_module_dotted_path() {
        assert_eq!(
            to_ast_string("module math.geometry\n"),
            "Program\n  ModuleDecl\n    ModulePath 'math.geometry'\n"
        );
    }

    #[test]
    fn test_module_dotted_path_long() {
        assert_eq!(
            to_ast_string("module app.utils.validation\n"),
            "Program\n  ModuleDecl\n    ModulePath 'app.utils.validation'\n"
        );
    }

    #[test]
    fn test_module_submodule() {
        assert_eq!(
            to_ast_string("module .utils\n"),
            "Program\n  ModuleDecl\n    ModulePath '.utils'\n"
        );
    }

    #[test]
    fn test_module_submodule_no_dots_after() {
        // Submodules can't have dots after the leading dot
        assert_eq!(
            to_ast_string("module .helpers\n"),
            "Program\n  ModuleDecl\n    ModulePath '.helpers'\n"
        );
    }

    #[test]
    fn test_error_module_missing_name() {
        assert!(to_ast("module\n").is_err());
    }

    #[test]
    fn test_error_module_invalid_name() {
        assert!(to_ast("module 123\n").is_err());
    }

    #[test]
    fn test_error_module_dot_only() {
        assert!(to_ast("module .\n").is_err());
    }

    // ========== Export Statement Tests ==========

    #[test]
    fn test_export_empty() {
        assert_eq!(
            to_ast_string("export {}\n"),
            "Program\n  Export\n    ExportList\n"
        );
    }

    #[test]
    fn test_export_single() {
        assert_eq!(
            to_ast_string("export { Calculator }\n"),
            "Program\n  Export\n    ExportList\n      Identifier 'Calculator'\n"
        );
    }

    #[test]
    fn test_export_multiple() {
        assert_eq!(
            to_ast_string("export { Calculator, add, subtract }\n"),
            "Program\n  Export\n    ExportList\n      Identifier 'Calculator'\n      Identifier 'add'\n      Identifier 'subtract'\n"
        );
    }

    #[test]
    fn test_export_with_newlines() {
        let source = r#"export {
    Calculator
    add
    subtract
}
"#;
        let output = to_ast_string(source);
        assert!(output.contains("ExportList"));
        assert!(output.contains("Identifier 'Calculator'"));
        assert!(output.contains("Identifier 'add'"));
        assert!(output.contains("Identifier 'subtract'"));
    }

    #[test]
    fn test_export_trailing_comma() {
        assert_eq!(
            to_ast_string("export { Calculator, add, }\n"),
            "Program\n  Export\n    ExportList\n      Identifier 'Calculator'\n      Identifier 'add'\n"
        );
    }

    #[test]
    fn test_error_export_missing_brace() {
        assert!(to_ast("export Calculator\n").is_err());
    }

    #[test]
    fn test_error_export_unclosed_brace() {
        assert!(to_ast("export { Calculator\n").is_err());
    }

    #[test]
    fn test_error_export_invalid_identifier() {
        assert!(to_ast("export { 123 }\n").is_err());
    }

    // ========== Import Statement Tests ==========

    #[test]
    fn test_import_empty() {
        assert_eq!(
            to_ast_string("import {}\n"),
            "Program\n  Import\n    ImportList\n"
        );
    }

    #[test]
    fn test_import_full_single() {
        assert_eq!(
            to_ast_string("import { math }\n"),
            "Program\n  Import\n    ImportList\n      ImportItem\n        Identifier 'math'\n"
        );
    }

    #[test]
    fn test_import_full_multiple() {
        let output = to_ast_string("import { math, io }\n");
        assert!(output.contains("ImportList"));
        assert!(output.contains("Identifier 'math'"));
        assert!(output.contains("Identifier 'io'"));
    }

    #[test]
    fn test_import_aliased() {
        assert_eq!(
            to_ast_string("import { m: math }\n"),
            "Program\n  Import\n    ImportList\n      ImportItem\n        ImportAlias 'm'\n        Identifier 'math'\n"
        );
    }

    #[test]
    fn test_import_selective_single() {
        assert_eq!(
            to_ast_string("import { {sin}: math }\n"),
            "Program\n  Import\n    ImportList\n      ImportItem\n        ImportSelective\n          ImportSelector 'sin'\n        Identifier 'math'\n"
        );
    }

    #[test]
    fn test_import_selective_multiple() {
        let output = to_ast_string("import { {sin, cos, pi}: math }\n");
        assert!(output.contains("ImportSelective"));
        assert!(output.contains("ImportSelector 'sin'"));
        assert!(output.contains("ImportSelector 'cos'"));
        assert!(output.contains("ImportSelector 'pi'"));
        assert!(output.contains("Identifier 'math'"));
    }

    #[test]
    fn test_import_selective_trailing_comma() {
        let output = to_ast_string("import { {sin, cos,}: math }\n");
        assert!(output.contains("ImportSelector 'sin'"));
        assert!(output.contains("ImportSelector 'cos'"));
    }

    #[test]
    fn test_import_star() {
        assert_eq!(
            to_ast_string("import { *: math }\n"),
            "Program\n  Import\n    ImportList\n      ImportItem\n        Identifier '*'\n        Identifier 'math'\n"
        );
    }

    #[test]
    fn test_import_mixed() {
        let source = r#"import {
    math
    m: trigonometry
    {sin, cos}: angles
    *: io
}
"#;
        let output = to_ast_string(source);
        assert!(output.contains("ImportList"));
        // Full import
        assert!(output.contains("Identifier 'math'"));
        // Aliased import
        assert!(output.contains("ImportAlias 'm'"));
        assert!(output.contains("Identifier 'trigonometry'"));
        // Selective import
        assert!(output.contains("ImportSelective"));
        assert!(output.contains("ImportSelector 'sin'"));
        assert!(output.contains("ImportSelector 'cos'"));
        assert!(output.contains("Identifier 'angles'"));
        // Star import
        assert!(output.contains("Identifier '*'"));
        assert!(output.contains("Identifier 'io'"));
    }

    #[test]
    fn test_import_dotted_module_path() {
        assert_eq!(
            to_ast_string("import { {sin}: math.trigonometry }\n"),
            "Program\n  Import\n    ImportList\n      ImportItem\n        ImportSelective\n          ImportSelector 'sin'\n        Identifier 'math.trigonometry'\n"
        );
    }

    #[test]
    fn test_import_full_dotted_path() {
        assert_eq!(
            to_ast_string("import { math.geometry }\n"),
            "Program\n  Import\n    ImportList\n      ImportItem\n        Identifier 'math.geometry'\n"
        );
    }

    #[test]
    fn test_import_trailing_comma() {
        let output = to_ast_string("import { math, io, }\n");
        assert!(output.contains("Identifier 'math'"));
        assert!(output.contains("Identifier 'io'"));
    }

    #[test]
    fn test_error_import_missing_brace() {
        assert!(to_ast("import math\n").is_err());
    }

    #[test]
    fn test_error_import_unclosed_brace() {
        assert!(to_ast("import { math\n").is_err());
    }

    #[test]
    fn test_error_import_missing_colon_aliased() {
        assert!(to_ast("import { m math }\n").is_err());
    }

    #[test]
    fn test_error_import_missing_colon_selective() {
        assert!(to_ast("import { {sin} math }\n").is_err());
    }

    #[test]
    fn test_error_import_missing_colon_star() {
        assert!(to_ast("import { * math }\n").is_err());
    }

    #[test]
    fn test_error_import_unclosed_selective() {
        assert!(to_ast("import { {sin: math }\n").is_err());
    }

    // ========== Integration Tests ==========

    #[test]
    fn test_complete_module_file() {
        let source = r#"module Calculator

import {
    math
    {sin, cos}: trigonometry
}

export {
    Calculator
    add
}

add: (x Number, y Number) Number {
    return x
}
"#;
        let ast = to_ast(source).unwrap();
        let output = ast.to_string();

        // Verify module, import, export, and function declarations are all present
        assert!(output.contains("ModuleDecl"));
        assert!(output.contains("Import"));
        assert!(output.contains("Export"));
        assert!(output.contains("FunctionDecl"));
    }

    #[test]
    fn test_submodule_with_imports() {
        let source = r#"module .utils

import {
    {isValid}: validation
}

export {
    formatNumber
}
"#;
        let ast = to_ast(source).unwrap();
        let output = ast.to_string();

        assert!(output.contains("ModulePath '.utils'"));
        assert!(output.contains("ImportSelector 'isValid'"));
        assert!(output.contains("Identifier 'formatNumber'"));
    }

    #[test]
    fn test_multiple_imports_and_exports() {
        let source = r#"module Library

import {
    math
    io
}

export {
    version
    init
}
"#;
        let ast = to_ast(source).unwrap();
        let output = ast.to_string();

        assert!(output.contains("ModuleDecl"));
        assert!(output.contains("Identifier 'math'"));
        assert!(output.contains("Identifier 'io'"));
        assert!(output.contains("Identifier 'version'"));
        assert!(output.contains("Identifier 'init'"));
    }
}

