// Parser module - splits parser into logical components
mod error;
mod expressions;
mod helpers;
mod list;
mod r#match;
mod statements;
mod struct_init;
mod types;

// Public exports
pub use error::ParseError;

use crate::ast::{Ast, AstNode, NodeType};
use crate::lexer::Tokens;

// Parser structure
pub struct Parser<'a> {
    tokens: Tokens,
    current: usize,
    ast: Ast,
    limits: &'a crate::limits::CompilerLimits,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Tokens, limits: &'a crate::limits::CompilerLimits) -> Self {
        let mut ast = Ast::new(tokens.string_storage.clone(), limits.clone());

        // Create the Program root node
        let program_node = AstNode::new(NodeType::Program);
        let root_idx = ast.add_node(program_node);
        ast.root = Some(root_idx);

        Self {
            tokens,
            current: 0,
            ast,
            limits,
        }
    }

    // Main parsing entry point
    pub fn parse(mut self) -> Result<Ast, ParseError> {
        self.parse_statements(0)?;
        Ok(self.ast)
    }
}

// Public API function
pub fn parse(tokens: Tokens, limits: &crate::limits::CompilerLimits) -> Result<Ast, ParseError> {
    let parser = Parser::new(tokens, limits);
    parser.parse()
}
