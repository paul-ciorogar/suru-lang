// Parser module - splits parser into logical components
mod error;
mod expressions;
mod helpers;
mod statements;
mod types;

// Public exports
pub use error::ParseError;

use crate::ast::{Ast, AstNode, NodeType};
use crate::lexer::{Token, TokenKind};

// Parser structure
pub struct Parser<'a> {
    source: &'a str,
    tokens: &'a Vec<Token>,
    current: usize,
    ast: Ast,
    limits: crate::limits::CompilerLimits,
}

impl<'a> Parser<'a> {
    pub fn new(
        source: &'a str,
        tokens: &'a Vec<Token>,
        limits: crate::limits::CompilerLimits,
    ) -> Self {
        let mut ast = Ast::new_with_limits(source.to_string(), limits.clone());

        // Create the Program root node
        let program_node = AstNode::new(NodeType::Program);
        let root_idx = ast.add_node(program_node);
        ast.root = Some(root_idx);

        Self {
            source,
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
pub fn parse(
    source: &str,
    tokens: &Vec<Token>,
    limits: crate::limits::CompilerLimits,
) -> Result<Ast, ParseError> {
    let parser = Parser::new(source, tokens, limits);
    parser.parse()
}
