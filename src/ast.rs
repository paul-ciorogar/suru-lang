use crate::lexer::Token;
use crate::string_storage::StringStorage;

// AST with single vector storage using first-child/next-sibling tree
#[derive(Debug)]
pub struct Ast {
    pub nodes: Vec<AstNode>,
    pub string_storage: StringStorage,
    pub root: Option<usize>, // Index of root node (usually a Program node)
    limits: crate::limits::CompilerLimits,
}

// Node types in the parse tree
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeType {
    Program,        // Root node containing all declarations
    VarDecl,        // Variable declaration
    Identifier,     // Identifier (terminal)
    LiteralBoolean, // Boolean literal (terminal)
    LiteralNumber,  // Number literal (terminal)
    LiteralString,  // String literal (terminal)

    // Boolean operations
    Not, // Unary not operation
    And, // Binary and operation
    Or,  // Binary or operation

    // Pipe operator
    Pipe, // Pipe operation: left | right

    // Function call
    FunctionCall,   // Function call expression
    MethodCall,     // Method call: receiver.method(args)
    PropertyAccess, // Property access: receiver.property
    ArgList,        // Argument list for function/method calls

    // Function declaration
    FunctionDecl,   // Function declaration: name: () { ... }
    ParamList,      // Parameter list: (x Type, y Type, ...)
    Param,          // Parameter: name Type or just name
    TypeAnnotation, // Type annotation (terminal - references type name)
    Block,          // Block of statements { stmt1 stmt2 ... }
    ExprStmt,       // Expression used as statement (wraps standalone calls)
    ReturnStmt,     // Return statement: return expr

    // Type declarations
    TypeDecl,           // Type declaration: type Name { ... }
    TypeName,           // Type name (identifier)
    TypeBody,   // Type body (unit, alias, union, struct, intersection, function, or generic)
    TypeParams, // Generic type parameters: <T, K, V>
    TypeParam,  // Single type parameter: T or T: Constraint
    TypeConstraint, // Type constraint in generics (terminal)
    UnionTypeList, // Union type alternatives: A, B, C
    StructBody, // Struct body: { fields and methods }
    StructField, // Struct field: name Type
    StructMethod, // Struct method declaration
    IntersectionType, // Type intersection using +
    FunctionType, // Function type signature
    FunctionTypeParams, // Function type parameter list
}

// Uniform-size parse tree node using first-child/next-sibling representation
#[derive(Debug, Clone)]
pub struct AstNode {
    pub node_type: NodeType,

    // Token information (for terminal nodes and position tracking)
    pub token: Option<Token>, // Full token (not just index)

    // Tree structure using indices
    pub first_child: Option<usize>,
    pub next_sibling: Option<usize>,
    pub parent: Option<usize>,
}

impl AstNode {
    pub fn new(node_type: NodeType) -> Self {
        Self {
            node_type,
            token: None,
            first_child: None,
            next_sibling: None,
            parent: None,
        }
    }

    pub fn new_terminal(node_type: NodeType, token: Token) -> Self {
        Self {
            node_type,
            token: Some(token),
            first_child: None,
            next_sibling: None,
            parent: None,
        }
    }
}

impl Ast {
    pub fn new(string_storage: StringStorage, limits: crate::limits::CompilerLimits) -> Self {
        Self {
            nodes: Vec::new(),
            string_storage,
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
    pub fn node_text(&self, node_idx: usize) -> Option<&str> {
        if let Some(ref token) = self.nodes[node_idx].token {
            token.text(&self.string_storage)
        } else {
            None
        }
    }

    // Return the AST tree structure as a string
    pub fn to_string(&self) -> String {
        if let Some(root_idx) = self.root {
            self.tree_string_recursive(root_idx, 0)
        } else {
            String::new()
        }
    }

    // Helper to format tree recursively as string
    fn tree_string_recursive(&self, node_idx: usize, depth: usize) -> String {
        let node = &self.nodes[node_idx];
        let indent = "  ".repeat(depth);

        let text = self
            .node_text(node_idx)
            .map(|s| format!(" '{}'", s))
            .or_else(|| {
                // Handle boolean keywords that aren't interned
                if let Some(ref token) = node.token {
                    use crate::lexer::TokenKind;
                    match token.kind {
                        TokenKind::True => Some(" 'true'".to_string()),
                        TokenKind::False => Some(" 'false'".to_string()),
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let mut result = format!("{}{:?}{}\n", indent, node.node_type, text);

        // Add children
        if let Some(child_idx) = node.first_child {
            let mut current = child_idx;
            loop {
                result.push_str(&self.tree_string_recursive(current, depth + 1));
                if let Some(next) = self.nodes[current].next_sibling {
                    current = next;
                } else {
                    break;
                }
            }
        }

        result
    }
}
