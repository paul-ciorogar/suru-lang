use crate::lexer::Token;
use crate::string_storage::StringStorage;
use bitflags::bitflags;

bitflags! {
    /// Flags for AST node metadata (privacy, mutability, etc.)
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct NodeFlags: u8 {
        const IS_PRIVATE = 0b00000001;
    }
}

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
    Placeholder,    // Placeholder _ for partial application (terminal)

    // Boolean operations
    Not, // Unary not operation
    And, // Binary and operation
    Or,  // Binary or operation

    // Arithmetic operations
    Negate, // Unary negation operation (-)

    // Error handling
    Try, // Unary try operation

    // Partial application
    Partial, // Unary partial application operation

    // Pipe operator
    Pipe, // Pipe operation: left | right

    // Composition operator
    Compose, // Composition operation: left + right

    // List literal
    List, // List literal: [elem1, elem2, ...]

    // Function call
    FunctionCall,   // Function call expression
    MethodCall,     // Method call: receiver.method(args)
    PropertyAccess, // Property access: receiver.property
    ArgList,        // Argument list for function/method calls

    // Match expressions
    Match,        // Match expression: match expr { arms }
    MatchSubject, // Subject expression being matched
    MatchArms,    // Container for all match arms
    MatchArm,     // Single match arm: pattern : result
    MatchPattern, // Pattern wrapper (identifier, literal, method call, or wildcard)

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

    // Struct initialization
    StructInit,       // Struct literal: { fields and methods }
    StructInitField,  // Field initialization: name: value
    StructInitMethod, // Method initialization: name: (params) { body }
    This,             // The 'this' keyword (terminal)

    // Module system
    ModuleDecl,       // Module declaration: module Name or module .name
    ModulePath,       // Module path/name (terminal - can be simple or dotted)
    Import,           // Import statement container
    ImportList,       // List of import items in {...}
    ImportItem,       // Single import item (full, aliased, selective, or star)
    ImportAlias,      // Alias in aliased import (terminal)
    ImportSelective,  // Selective import list {...}
    ImportSelector,   // Single selector in selective import (terminal)
    Export,           // Export statement container
    ExportList,       // List of exported identifiers
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

    // Node metadata flags (privacy, mutability, etc.)
    pub flags: NodeFlags,
}

impl AstNode {
    pub fn new(node_type: NodeType) -> Self {
        Self {
            node_type,
            token: None,
            first_child: None,
            next_sibling: None,
            parent: None,
            flags: NodeFlags::empty(),
        }
    }

    pub fn new_terminal(node_type: NodeType, token: Token) -> Self {
        Self {
            node_type,
            token: Some(token),
            first_child: None,
            next_sibling: None,
            parent: None,
            flags: NodeFlags::empty(),
        }
    }

    pub fn new_private(node_type: NodeType) -> Self {
        Self {
            node_type,
            token: None,
            first_child: None,
            next_sibling: None,
            parent: None,
            flags: NodeFlags::IS_PRIVATE,
        }
    }

    pub fn new_private_terminal(node_type: NodeType, token: Token) -> Self {
        Self {
            node_type,
            token: Some(token),
            first_child: None,
            next_sibling: None,
            parent: None,
            flags: NodeFlags::IS_PRIVATE,
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

    /// Iterate over direct children of a node (first-child/next-sibling traversal)
    pub fn children(&self, node_idx: usize) -> ChildIter<'_> {
        ChildIter {
            ast: self,
            current: self.nodes[node_idx].first_child,
        }
    }

    /// Get a typed view over a VarDecl node
    pub fn var_decl(&self, node_idx: usize) -> VarDeclView<'_> {
        VarDeclView { ast: self, idx: node_idx }
    }

    /// Get a typed view over a FunctionDecl node
    pub fn function_decl(&self, node_idx: usize) -> FunctionDeclView<'_> {
        FunctionDeclView { ast: self, idx: node_idx }
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
                        TokenKind::Underscore => Some(" '_'".to_string()),
                        TokenKind::This => Some(" 'this'".to_string()),
                        TokenKind::Star => Some(" '*'".to_string()),
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .unwrap_or_default();

        // Add privacy marker if node is private
        let privacy_marker = if node.flags.contains(NodeFlags::IS_PRIVATE) {
            " [private]"
        } else {
            ""
        };

        let mut result = format!("{}{:?}{}{}\n", indent, node.node_type, text, privacy_marker);

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

// =============================================================================
// AST View Types (Flyweight Pattern)
//
// Thin, non-owning wrappers around node indices that provide semantic access
// to AST nodes without copying data. Each view holds a reference to the Ast
// and the index of the node it represents.
// =============================================================================

/// Iterator over direct child nodes (first-child/next-sibling traversal)
pub struct ChildIter<'a> {
    ast: &'a Ast,
    current: Option<usize>,
}

impl<'a> Iterator for ChildIter<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        let idx = self.current?;
        self.current = self.ast.nodes[idx].next_sibling;
        Some(idx)
    }
}

/// View over a `VarDecl` node
///
/// AST structure:
/// ```text
/// VarDecl
///   Identifier 'name'
///   TypeAnnotation 'Type'   (optional)
///   <Expression>            (value)
/// ```
pub struct VarDeclView<'a> {
    ast: &'a Ast,
    idx: usize,
}

impl<'a> VarDeclView<'a> {
    /// Returns the variable name from the first child Identifier
    pub fn name(&self) -> Option<&str> {
        let ident_idx = self.ast.nodes[self.idx].first_child?;
        self.ast.node_text(ident_idx)
    }

    /// Returns the index of the Identifier node (for token/error reporting)
    pub fn ident_idx(&self) -> Option<usize> {
        self.ast.nodes[self.idx].first_child
    }

    /// Returns the type annotation string if present (second child when TypeAnnotation)
    pub fn type_annotation(&self) -> Option<&str> {
        let ident_idx = self.ast.nodes[self.idx].first_child?;
        let second_idx = self.ast.nodes[ident_idx].next_sibling?;
        if self.ast.nodes[second_idx].node_type == NodeType::TypeAnnotation {
            self.ast.node_text(second_idx)
        } else {
            None
        }
    }

    /// Returns the index of the value expression node if present.
    /// Skips the type annotation if one exists.
    pub fn value_expr_idx(&self) -> Option<usize> {
        let ident_idx = self.ast.nodes[self.idx].first_child?;
        let second_idx = self.ast.nodes[ident_idx].next_sibling?;
        if self.ast.nodes[second_idx].node_type == NodeType::TypeAnnotation {
            self.ast.nodes[second_idx].next_sibling
        } else {
            Some(second_idx)
        }
    }
}

/// View over a single `Param` node
///
/// AST structure:
/// ```text
/// Param
///   Identifier 'name'
///   TypeAnnotation 'Type'   (optional)
/// ```
pub struct ParamView<'a> {
    ast: &'a Ast,
    idx: usize,
}

impl<'a> ParamView<'a> {
    /// Returns the node index of this Param
    pub fn idx(&self) -> usize {
        self.idx
    }

    /// Returns the parameter name from the first child Identifier
    pub fn name(&self) -> Option<&str> {
        let ident_idx = self.ast.nodes[self.idx].first_child?;
        self.ast.node_text(ident_idx)
    }

    /// Returns the type annotation string if present
    pub fn type_annotation(&self) -> Option<&str> {
        let ident_idx = self.ast.nodes[self.idx].first_child?;
        let type_ann_idx = self.ast.nodes[ident_idx].next_sibling?;
        if self.ast.nodes[type_ann_idx].node_type == NodeType::TypeAnnotation {
            self.ast.node_text(type_ann_idx)
        } else {
            None
        }
    }
}

/// Iterator over `Param` nodes in a `ParamList`
pub struct ParamIter<'a> {
    ast: &'a Ast,
    current: Option<usize>,
}

impl<'a> Iterator for ParamIter<'a> {
    type Item = ParamView<'a>;

    fn next(&mut self) -> Option<ParamView<'a>> {
        let idx = self.current?;
        self.current = self.ast.nodes[idx].next_sibling;
        Some(ParamView { ast: self.ast, idx })
    }
}

/// View over a `FunctionDecl` node
///
/// AST structure:
/// ```text
/// FunctionDecl
///   Identifier 'name'
///   TypeParams              (optional)
///   ParamList
///     Param*
///   TypeAnnotation          (return type, optional)
///   Block
/// ```
pub struct FunctionDeclView<'a> {
    ast: &'a Ast,
    idx: usize,
}

impl<'a> FunctionDeclView<'a> {
    /// Returns the function name from the first child Identifier
    pub fn name(&self) -> Option<&str> {
        let ident_idx = self.ast.nodes[self.idx].first_child?;
        self.ast.node_text(ident_idx)
    }

    /// Returns the index of the Identifier node (for token/error reporting)
    pub fn ident_idx(&self) -> Option<usize> {
        self.ast.nodes[self.idx].first_child
    }

    /// Returns the index of the TypeParams node if present (generic functions)
    pub fn type_params_idx(&self) -> Option<usize> {
        let ident_idx = self.ast.nodes[self.idx].first_child?;
        let next_idx = self.ast.nodes[ident_idx].next_sibling?;
        if self.ast.nodes[next_idx].node_type == NodeType::TypeParams {
            Some(next_idx)
        } else {
            None
        }
    }

    /// Returns the index of the ParamList node, skipping optional TypeParams
    pub fn param_list_idx(&self) -> Option<usize> {
        let ident_idx = self.ast.nodes[self.idx].first_child?;
        let next_idx = self.ast.nodes[ident_idx].next_sibling?;
        if self.ast.nodes[next_idx].node_type == NodeType::TypeParams {
            self.ast.nodes[next_idx].next_sibling
        } else if self.ast.nodes[next_idx].node_type == NodeType::ParamList {
            Some(next_idx)
        } else {
            None
        }
    }

    /// Returns the return type annotation string if present (TypeAnnotation after ParamList)
    pub fn return_type_annotation(&self) -> Option<&str> {
        let param_list_idx = self.param_list_idx()?;
        let after_params_idx = self.ast.nodes[param_list_idx].next_sibling?;
        if self.ast.nodes[after_params_idx].node_type == NodeType::TypeAnnotation {
            self.ast.node_text(after_params_idx)
        } else {
            None
        }
    }

    /// Returns the index of the Block (function body) node if present
    pub fn body_idx(&self) -> Option<usize> {
        let param_list_idx = self.param_list_idx()?;
        let mut current = Some(param_list_idx);
        while let Some(idx) = current {
            if self.ast.nodes[idx].node_type == NodeType::Block {
                return Some(idx);
            }
            current = self.ast.nodes[idx].next_sibling;
        }
        None
    }

    /// Iterates over the Param nodes in the ParamList
    pub fn params(&self) -> ParamIter<'a> {
        let first_param = self
            .param_list_idx()
            .and_then(|pl| self.ast.nodes[pl].first_child);
        ParamIter { ast: self.ast, current: first_param }
    }
}
