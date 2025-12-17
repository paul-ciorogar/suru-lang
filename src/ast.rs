use crate::lexer::Token;

// AST with single vector storage using first-child/next-sibling tree
#[derive(Debug)]
pub struct Ast {
    pub nodes: Vec<AstNode>,
    pub source: String,
    pub root: Option<usize>, // Index of root node (usually a Program node)
    limits: crate::limits::CompilerLimits,
}

// Node types in the parse tree
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeType {
    Program,        // Root node containing all declarations
    VarDecl,        // Variable declaration
    Ident,          // Identifier (terminal)
    LiteralBoolean, // Boolean literal (terminal)
    LiteralNumber,  // Number literal (terminal)
    LiteralString,  // String literal (terminal)

    // Boolean operations
    Not, // Unary not operation
    And, // Binary and operation
    Or,  // Binary or operation
}

// Uniform-size parse tree node using first-child/next-sibling representation
#[derive(Debug, Clone)]
pub struct AstNode {
    pub node_type: NodeType,

    // Token information (for terminal nodes and position tracking)
    pub token_idx: Option<usize>, // Index into token stream

    // Tree structure using indices (None = -1 in C)
    // Tree structure using indices
    pub first_child: Option<usize>,
    pub next_sibling: Option<usize>,
    pub parent: Option<usize>,
}

impl AstNode {
    pub fn new(node_type: NodeType) -> Self {
        Self {
            node_type,
            token_idx: None,
            first_child: None,
            next_sibling: None,
            parent: None,
        }
    }

    pub fn new_terminal(node_type: NodeType, token_idx: usize) -> Self {
        Self {
            node_type,
            token_idx: Some(token_idx),
            first_child: None,
            next_sibling: None,
            parent: None,
        }
    }
}

impl Ast {
    pub fn new(source: String) -> Self {
        Self::new_with_limits(source, crate::limits::CompilerLimits::default())
    }

    pub fn new_with_limits(source: String, limits: crate::limits::CompilerLimits) -> Self {
        Self {
            nodes: Vec::new(),
            source,
            root: None,
            limits,
        }
    }

    // Add node and return its index
    pub fn add_node(&mut self, node: AstNode) -> usize {
        // Check AST node limit before adding
        if self.nodes.len() >= self.limits.max_ast_nodes {
            panic!(
                "AST node limit exceeded: {} nodes (max: {}). File is too complex.",
                self.nodes.len(),
                self.limits.max_ast_nodes
            );
        }

        let idx = self.nodes.len();
        self.nodes.push(node);
        idx
    }

    // Link child to parent (adds as last child)
    pub fn add_child(&mut self, parent_idx: usize, child_idx: usize) {
        self.nodes[child_idx].parent = Some(parent_idx);

        if let Some(first_child_idx) = self.nodes[parent_idx].first_child {
            // Find last sibling and append
            let mut current = first_child_idx;
            while let Some(next) = self.nodes[current].next_sibling {
                current = next;
            }
            self.nodes[current].next_sibling = Some(child_idx);
        } else {
            // This is the first child
            self.nodes[parent_idx].first_child = Some(child_idx);
        }
    }

    // Get node text from token
    pub fn node_text(&self, node_idx: usize, tokens: &[Token]) -> Option<&str> {
        if let Some(token_idx) = self.nodes[node_idx].token_idx {
            Some(tokens[token_idx].text(&self.source))
        } else {
            None
        }
    }
}
