use std::collections::HashMap;

mod name_resolution;
mod type_resolution;
mod type_inference;
mod expression_type_inference;
mod unification;
mod types;
mod assignment_type_checking;

pub use types::{
    Type, TypeId, TypeRegistry,
    IntSize, UIntSize, FloatSize,
    StructType, StructField, StructMethod,
    FunctionType, FunctionParam,
    // Hindley-Milner type inference
    TypeVarId, Constraint, Substitution,
};

/// Represents a semantic analysis error
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl SemanticError {
    pub fn new(message: String, line: usize, column: usize) -> Self {
        SemanticError {
            message,
            line,
            column,
        }
    }

    pub fn from_token(message: String, token: &crate::lexer::Token) -> Self {
        SemanticError {
            message,
            line: token.line,
            column: token.column,
        }
    }
}

impl std::fmt::Display for SemanticError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Semantic error at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for SemanticError {}

/// Represents the kind of symbol in the symbol table
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Variable,
    Function,
    Type,
}

/// Represents the kind of scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    Global,   // File-level scope
    Module,   // Module scope
    Function, // Function body scope
    Block,    // Block scope (nested blocks, match arms)
}

/// Represents a symbol in the symbol table
#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    /// The name of the symbol
    pub name: String,
    /// The type of the symbol as a string (if known). Will be None for untyped or type-inferred symbols
    /// For functions, contains signature like "(Number, String) -> Bool"
    pub type_name: Option<String>,
    /// The kind of symbol (variable, function, or type)
    pub kind: SymbolKind,
    /// The structured type ID for type checking (Phase 5.1+)
    /// For functions, contains the interned FunctionType
    pub type_id: Option<TypeId>,
}

impl Symbol {
    /// Creates a new symbol
    pub fn new(name: String, type_name: Option<String>, kind: SymbolKind) -> Self {
        Symbol {
            name,
            type_name,
            kind,
            type_id: None,
        }
    }

    /// Builder method to set the structured type ID
    pub fn with_type_id(mut self, type_id: TypeId) -> Self {
        self.type_id = Some(type_id);
        self
    }
}

/// Represents a single scope in the scope hierarchy
#[derive(Debug, Clone)]
pub struct Scope {
    /// The kind of this scope
    pub kind: ScopeKind,
    /// The symbol table for this scope
    pub symbols: SymbolTable,
    /// Index of parent scope (None for global scope)
    pub parent: Option<usize>,
}

impl Scope {
    /// Creates a new scope with the given kind and parent
    pub fn new(kind: ScopeKind, parent: Option<usize>) -> Self {
        Scope {
            kind,
            symbols: SymbolTable::new(),
            parent,
        }
    }

    /// Inserts a symbol into this scope's symbol table
    /// Returns true if inserted, false if already exists
    pub fn insert_symbol(&mut self, symbol: Symbol) -> bool {
        self.symbols.insert(symbol)
    }

    /// Looks up a symbol in this scope only (does not check parent)
    pub fn lookup_local(&self, name: &str) -> Option<&Symbol> {
        self.symbols.lookup(name)
    }
}

/// Symbol table for storing and looking up symbols in a scope
#[derive(Debug, Clone)]
pub struct SymbolTable {
    /// Map from symbol name to symbol information
    symbols: HashMap<String, Symbol>,
}

impl SymbolTable {
    /// Creates a new empty symbol table
    pub fn new() -> Self {
        SymbolTable {
            symbols: HashMap::new(),
        }
    }

    /// Inserts a symbol into the table
    /// Returns true if the symbol was newly inserted, false if it already existed
    pub fn insert(&mut self, symbol: Symbol) -> bool {
        let name = symbol.name.clone();
        if self.symbols.contains_key(&name) {
            false
        } else {
            self.symbols.insert(name, symbol);
            true
        }
    }

    /// Inserts or replaces a symbol in the table
    /// Always succeeds, overwriting any existing symbol with the same name
    /// Used for variable redeclaration support
    pub fn insert_or_replace(&mut self, symbol: Symbol) {
        let name = symbol.name.clone();
        self.symbols.insert(name, symbol);
    }

    /// Looks up a symbol by name
    /// Returns Some(&Symbol) if found, None otherwise
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name)
    }

    /// Checks if a symbol exists in the table
    pub fn contains(&self, name: &str) -> bool {
        self.symbols.contains_key(name)
    }

    /// Returns the number of symbols in the table
    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    /// Returns true if the symbol table is empty
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Manages a stack of nested scopes
#[derive(Debug)]
pub struct ScopeStack {
    /// Arena of all scopes (indexed by scope id)
    scopes: Vec<Scope>,
    /// Stack of current scope indices (top = current scope)
    current_stack: Vec<usize>,
}

impl ScopeStack {
    /// Creates a new scope stack with a global scope
    pub fn new() -> Self {
        let mut scopes = Vec::new();
        let global_scope = Scope::new(ScopeKind::Global, None);
        scopes.push(global_scope);

        ScopeStack {
            scopes,
            current_stack: vec![0], // Start with global scope active
        }
    }

    /// Enters a new scope of the given kind
    /// Returns the index of the newly created scope
    pub fn enter_scope(&mut self, kind: ScopeKind) -> usize {
        let parent_idx = *self.current_stack.last().unwrap();
        let new_scope = Scope::new(kind, Some(parent_idx));
        let scope_idx = self.scopes.len();
        self.scopes.push(new_scope);
        self.current_stack.push(scope_idx);
        scope_idx
    }

    /// Exits the current scope
    /// Returns the index of the exited scope
    /// Panics if trying to exit the global scope
    pub fn exit_scope(&mut self) -> usize {
        assert!(self.current_stack.len() > 1, "Cannot exit global scope");
        self.current_stack.pop().unwrap()
    }

    /// Returns the index of the current scope
    pub fn current_scope_index(&self) -> usize {
        *self.current_stack.last().unwrap()
    }

    /// Returns a reference to the current scope
    pub fn current_scope(&self) -> &Scope {
        let idx = self.current_scope_index();
        &self.scopes[idx]
    }

    /// Returns a mutable reference to the current scope
    pub fn current_scope_mut(&mut self) -> &mut Scope {
        let idx = self.current_scope_index();
        &mut self.scopes[idx]
    }

    /// Inserts a symbol into the current scope
    /// Returns true if inserted, false if already exists in current scope
    pub fn insert(&mut self, symbol: Symbol) -> bool {
        self.current_scope_mut().insert_symbol(symbol)
    }

    /// Looks up a symbol by searching the scope chain from current to global
    /// Returns Some(&Symbol) if found, None otherwise
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        // Start from current scope and walk up parent chain
        let mut current_idx = self.current_scope_index();
        loop {
            let scope = &self.scopes[current_idx];
            if let Some(symbol) = scope.lookup_local(name) {
                return Some(symbol);
            }

            // Move to parent scope
            match scope.parent {
                Some(parent_idx) => current_idx = parent_idx,
                None => return None, // Reached global scope, not found
            }
        }
    }

    /// Returns true if a symbol exists in the current scope chain
    pub fn contains(&self, name: &str) -> bool {
        self.lookup(name).is_some()
    }

    /// Returns the current scope depth (0 = global, 1 = first nested scope, etc.)
    pub fn depth(&self) -> usize {
        self.current_stack.len() - 1
    }

    /// Returns true if current scope is inside a function or block (mutable context)
    /// Variables declared in mutable scopes can be reassigned.
    pub fn is_in_mutable_scope(&self) -> bool {
        for &scope_idx in &self.current_stack {
            match self.scopes[scope_idx].kind {
                ScopeKind::Function | ScopeKind::Block => return true,
                _ => {}
            }
        }
        false
    }
}

impl Default for ScopeStack {
    fn default() -> Self {
        Self::new()
    }
}

/// Main semantic analyzer that traverses AST and performs semantic checks
pub struct SemanticAnalyzer {
    ast: crate::ast::Ast,
    scopes: ScopeStack,
    type_registry: TypeRegistry,
    errors: Vec<SemanticError>,

    // Hindley-Milner type inference infrastructure
    /// Maps AST nodes to their inferred types
    node_types: HashMap<usize, TypeId>,
    /// Collected type constraints
    constraints: Vec<Constraint>,
    /// Current substitution (solution to constraints)
    substitution: Substitution,
    /// Counter for generating fresh type variables
    next_type_var: u32,

    // Assignment type checking (Phase 4.4)
    /// Maps (scope_index, variable_name) to their TypeId for reassignment checking
    variable_types: HashMap<(usize, String), TypeId>,
}

impl SemanticAnalyzer {
    /// Creates a new semantic analyzer with the given AST
    pub fn new(ast: crate::ast::Ast) -> Self {
        let mut type_registry = TypeRegistry::new();
        Self::register_builtin_types(&mut type_registry);

        SemanticAnalyzer {
            ast,
            scopes: ScopeStack::new(),
            type_registry,
            errors: Vec::new(),
            // Initialize Hindley-Milner infrastructure
            node_types: HashMap::new(),
            constraints: Vec::new(),
            substitution: Substitution::new(),
            next_type_var: 0,
            // Initialize assignment type checking
            variable_types: HashMap::new(),
        }
    }

    /// Records a semantic error
    fn record_error(&mut self, error: SemanticError) {
        self.errors.push(error);
    }

    // ========== Hindley-Milner Helper Methods ==========

    /// Generates a fresh type variable for inference
    ///
    /// Each call returns a unique type variable (e.g., '0, '1, '2, ...)
    /// Used when the type of an expression is unknown and must be inferred.
    fn fresh_type_var(&mut self) -> TypeId {
        let var_id = TypeVarId::new(self.next_type_var);
        self.next_type_var += 1;
        self.type_registry.intern(Type::Var(var_id))
    }

    /// Records the inferred type for an AST node
    fn set_node_type(&mut self, node_idx: usize, type_id: TypeId) {
        self.node_types.insert(node_idx, type_id);
    }

    /// Gets the inferred type for an AST node (if any)
    pub fn get_node_type(&self, node_idx: usize) -> Option<TypeId> {
        self.node_types.get(&node_idx).copied()
    }

    /// Adds a type equality constraint
    ///
    /// Constraints are collected during AST traversal and solved via unification.
    fn add_constraint(&mut self, left: TypeId, right: TypeId, source: usize) {
        self.constraints.push(Constraint::new(left, right, source));
    }

    // ========== Assignment Type Checking Helper Methods ==========

    /// Looks up a variable's declared type by searching the scope chain
    ///
    /// Used for reassignment type checking. Returns the TypeId of the variable
    /// if it was previously declared, or None if not found.
    fn lookup_variable_type(&self, name: &str) -> Option<TypeId> {
        let mut scope_idx = self.scopes.current_scope_index();
        loop {
            if let Some(&type_id) = self.variable_types.get(&(scope_idx, name.to_string())) {
                return Some(type_id);
            }
            match self.scopes.scopes[scope_idx].parent {
                Some(parent_idx) => scope_idx = parent_idx,
                None => return None,
            }
        }
    }

    /// Records a variable's type for future reassignment checking
    fn record_variable_type(&mut self, name: &str, type_id: TypeId) {
        let scope_idx = self.scopes.current_scope_index();
        self.variable_types.insert((scope_idx, name.to_string()), type_id);
    }

    /// Registers all built-in types in the type registry
    fn register_builtin_types(registry: &mut TypeRegistry) {
        // Primitive types
        registry.intern(Type::Unit);
        registry.intern(Type::Number);
        registry.intern(Type::String);
        registry.intern(Type::Bool);

        // Sized integers
        registry.intern(Type::Int(IntSize::I8));
        registry.intern(Type::Int(IntSize::I16));
        registry.intern(Type::Int(IntSize::I32));
        registry.intern(Type::Int(IntSize::I64));

        // Sized unsigned integers
        registry.intern(Type::UInt(UIntSize::U8));
        registry.intern(Type::UInt(UIntSize::U16));
        registry.intern(Type::UInt(UIntSize::U32));
        registry.intern(Type::UInt(UIntSize::U64));

        // Floats
        registry.intern(Type::Float(FloatSize::F32));
        registry.intern(Type::Float(FloatSize::F64));
    }

    /// Checks if a given name is a built-in type
    fn is_builtin_type(name: &str) -> bool {
        matches!(
            name,
            "Unit" | "Number" | "String" | "Bool" |
            "Int8" | "Int16" | "Int32" | "Int64" |
            "UInt8" | "UInt16" | "UInt32" | "UInt64" |
            "Float32" | "Float64"
        )
    }

    /// Checks if a type with the given name exists (built-in or user-defined)
    fn type_exists(&self, name: &str) -> bool {
        // Check built-in types first
        if Self::is_builtin_type(name) {
            return true;
        }

        // Check symbol table for user-defined types
        if let Some(symbol) = self.scopes.lookup(name) {
            return symbol.kind == SymbolKind::Type;
        }

        false
    }

    /// Looks up the TypeId for a given type name
    /// Returns an error if the type doesn't exist
    fn lookup_type_id(&mut self, name: &str) -> Result<TypeId, SemanticError> {
        // For built-in types, construct the Type and intern it
        // (Will return existing TypeId due to interning)
        let ty = match name {
            "Unit" => Type::Unit,
            "Number" => Type::Number,
            "String" => Type::String,
            "Bool" => Type::Bool,
            "Int8" => Type::Int(IntSize::I8),
            "Int16" => Type::Int(IntSize::I16),
            "Int32" => Type::Int(IntSize::I32),
            "Int64" => Type::Int(IntSize::I64),
            "UInt8" => Type::UInt(UIntSize::U8),
            "UInt16" => Type::UInt(UIntSize::U16),
            "UInt32" => Type::UInt(UIntSize::U32),
            "UInt64" => Type::UInt(UIntSize::U64),
            "Float32" => Type::Float(FloatSize::F32),
            "Float64" => Type::Float(FloatSize::F64),
            _ => {
                // Look up user-defined type in symbol table
                if let Some(symbol) = self.scopes.lookup(name) {
                    if symbol.kind == SymbolKind::Type {
                        // Extract TypeId from symbol.type_name
                        // Format is "TypeId(N)" where N is the index
                        if let Some(type_str) = &symbol.type_name {
                            if let Some(id_str) = type_str
                                .strip_prefix("TypeId(")
                                .and_then(|s| s.strip_suffix(")"))
                            {
                                if let Ok(id) = id_str.parse::<usize>() {
                                    return Ok(TypeId::new(id));
                                }
                            }
                        }
                    }
                }

                // Type not found
                return Err(SemanticError::new(
                    format!("Internal error: Type '{}' not found in registry", name),
                    0,
                    0,
                ));
            }
        };

        // For built-in types, intern and return
        Ok(self.type_registry.intern(ty))
    }

    /// Performs semantic analysis on the AST
    /// Returns Ok(Ast) if no errors, or Err(Vec<SemanticError>) if errors found
    ///
    /// # Algorithm (Hindley-Milner)
    ///
    /// 1. **Constraint Collection**: Traverse AST and collect type constraints
    /// 2. **Unification**: Solve constraints to find type substitution
    /// 3. **Substitution Application**: Apply final types to all nodes
    pub fn analyze(mut self) -> Result<crate::ast::Ast, Vec<SemanticError>> {
        if let Some(root_idx) = self.ast.root {
            // Phase 1: Collect constraints by traversing AST
            self.visit_node(root_idx);

            // Phase 2: Solve constraints via unification
            if let Err(errors) = self.solve_constraints() {
                self.errors.extend(errors);
            }

            // Phase 3: Apply final substitution to all node types
            self.apply_substitution();
        }

        if self.errors.is_empty() {
            Ok(self.ast)
        } else {
            Err(self.errors)
        }
    }

    /// Visits a node and dispatches to appropriate visitor method
    fn visit_node(&mut self, node_idx: usize) {
        use crate::ast::NodeType;

        let node = &self.ast.nodes[node_idx];

        match node.node_type {
            NodeType::Program => self.visit_program(node_idx),
            NodeType::VarDecl => self.visit_var_decl(node_idx),
            NodeType::FunctionDecl => self.visit_function_decl(node_idx),
            NodeType::TypeDecl => self.visit_type_decl(node_idx),
            NodeType::Block => self.visit_block(node_idx),
            NodeType::Identifier => self.visit_identifier(node_idx),
            NodeType::FunctionCall => self.visit_function_call(node_idx),
            // Type inference for literals (Hindley-Milner)
            NodeType::LiteralNumber => self.visit_literal_number(node_idx),
            NodeType::LiteralString => self.visit_literal_string(node_idx),
            NodeType::LiteralBoolean => self.visit_literal_boolean(node_idx),
            NodeType::List => self.visit_list(node_idx),
            // Type inference for operators (Phase 4.2)
            NodeType::And | NodeType::Or => self.visit_binary_bool_op(node_idx),
            NodeType::Not => self.visit_not(node_idx),
            NodeType::Negate => self.visit_negate(node_idx),
            // For now, just visit children for all other node types
            _ => self.visit_children(node_idx),
        }
    }

    /// Visits all children of a node
    fn visit_children(&mut self, node_idx: usize) {
        if let Some(first_child_idx) = self.ast.nodes[node_idx].first_child {
            let mut current = first_child_idx;
            loop {
                self.visit_node(current);

                if let Some(next) = self.ast.nodes[current].next_sibling {
                    current = next;
                } else {
                    break;
                }
            }
        }
    }

    /// Visits Program node (root)
    fn visit_program(&mut self, node_idx: usize) {
        // Program node just contains declarations at global scope
        self.visit_children(node_idx);
    }

    // Variable declaration and function declaration visitors are implemented in name_resolution.rs
    // Type declaration visitor is implemented in type_resolution.rs

    /// Visits block statement
    fn visit_block(&mut self, node_idx: usize) {
        // Enter new block scope
        self.scopes.enter_scope(ScopeKind::Block);

        // Visit all statements in the block
        self.visit_children(node_idx);

        // Exit block scope
        self.scopes.exit_scope();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_creation() {
        let symbol = Symbol::new(
            "x".to_string(),
            Some("Number".to_string()),
            SymbolKind::Variable,
        );

        assert_eq!(symbol.name, "x");
        assert_eq!(symbol.type_name, Some("Number".to_string()));
        assert_eq!(symbol.kind, SymbolKind::Variable);
    }

    #[test]
    fn test_symbol_without_type() {
        let symbol = Symbol::new("y".to_string(), None, SymbolKind::Variable);

        assert_eq!(symbol.name, "y");
        assert_eq!(symbol.type_name, None);
        assert_eq!(symbol.kind, SymbolKind::Variable);
    }

    #[test]
    fn test_symbol_table_insert_and_lookup() {
        let mut table = SymbolTable::new();

        let symbol = Symbol::new(
            "x".to_string(),
            Some("Number".to_string()),
            SymbolKind::Variable,
        );

        assert!(table.insert(symbol.clone()));
        assert_eq!(table.len(), 1);

        let found = table.lookup("x");
        assert!(found.is_some());
        assert_eq!(found.unwrap(), &symbol);
    }

    #[test]
    fn test_symbol_table_duplicate_insert() {
        let mut table = SymbolTable::new();

        let symbol1 = Symbol::new(
            "x".to_string(),
            Some("Number".to_string()),
            SymbolKind::Variable,
        );

        let symbol2 = Symbol::new(
            "x".to_string(),
            Some("String".to_string()),
            SymbolKind::Variable,
        );

        assert!(table.insert(symbol1.clone()));
        assert!(!table.insert(symbol2));
        assert_eq!(table.len(), 1);

        let found = table.lookup("x");
        assert_eq!(found.unwrap().type_name, Some("Number".to_string()));
    }

    #[test]
    fn test_symbol_table_lookup_nonexistent() {
        let table = SymbolTable::new();

        let found = table.lookup("nonexistent");
        assert!(found.is_none());
    }

    #[test]
    fn test_symbol_table_contains() {
        let mut table = SymbolTable::new();

        let symbol = Symbol::new(
            "x".to_string(),
            Some("Number".to_string()),
            SymbolKind::Variable,
        );

        assert!(!table.contains("x"));
        table.insert(symbol);
        assert!(table.contains("x"));
    }

    #[test]
    fn test_symbol_table_multiple_symbols() {
        let mut table = SymbolTable::new();

        let var_symbol = Symbol::new(
            "x".to_string(),
            Some("Number".to_string()),
            SymbolKind::Variable,
        );

        let func_symbol = Symbol::new(
            "foo".to_string(),
            Some("() -> Number".to_string()),
            SymbolKind::Function,
        );

        let type_symbol = Symbol::new("MyType".to_string(), None, SymbolKind::Type);

        assert!(table.insert(var_symbol));
        assert!(table.insert(func_symbol));
        assert!(table.insert(type_symbol));
        assert_eq!(table.len(), 3);

        assert!(table.contains("x"));
        assert!(table.contains("foo"));
        assert!(table.contains("MyType"));

        assert_eq!(table.lookup("x").unwrap().kind, SymbolKind::Variable);
        assert_eq!(table.lookup("foo").unwrap().kind, SymbolKind::Function);
        assert_eq!(table.lookup("MyType").unwrap().kind, SymbolKind::Type);
    }

    #[test]
    fn test_symbol_table_is_empty() {
        let mut table = SymbolTable::new();

        assert!(table.is_empty());

        let symbol = Symbol::new(
            "x".to_string(),
            Some("Number".to_string()),
            SymbolKind::Variable,
        );

        table.insert(symbol);
        assert!(!table.is_empty());
    }

    // Helper function for scope tests
    fn test_symbol(name: &str) -> Symbol {
        Symbol::new(
            name.to_string(),
            Some("Number".to_string()),
            SymbolKind::Variable,
        )
    }

    // ========== Basic Scope Operations ==========

    #[test]
    fn test_scope_creation() {
        let scope = Scope::new(ScopeKind::Function, Some(0));
        assert_eq!(scope.kind, ScopeKind::Function);
        assert_eq!(scope.parent, Some(0));
        assert!(scope.symbols.is_empty());
    }

    #[test]
    fn test_scope_stack_initialization() {
        let stack = ScopeStack::new();
        assert_eq!(stack.depth(), 0);
        assert_eq!(stack.current_scope().kind, ScopeKind::Global);
        assert!(stack.current_scope().parent.is_none());
    }

    #[test]
    fn test_scope_stack_enter_exit() {
        let mut stack = ScopeStack::new();

        // Enter function scope
        let func_idx = stack.enter_scope(ScopeKind::Function);
        assert_eq!(stack.depth(), 1);
        assert_eq!(stack.current_scope().kind, ScopeKind::Function);
        assert_eq!(stack.current_scope().parent, Some(0));

        // Exit function scope
        let exited_idx = stack.exit_scope();
        assert_eq!(exited_idx, func_idx);
        assert_eq!(stack.depth(), 0);
        assert_eq!(stack.current_scope().kind, ScopeKind::Global);
    }

    #[test]
    fn test_scope_stack_depth() {
        let mut stack = ScopeStack::new();

        assert_eq!(stack.depth(), 0);
        stack.enter_scope(ScopeKind::Module);
        assert_eq!(stack.depth(), 1);
        stack.enter_scope(ScopeKind::Function);
        assert_eq!(stack.depth(), 2);
        stack.exit_scope();
        assert_eq!(stack.depth(), 1);
        stack.exit_scope();
        assert_eq!(stack.depth(), 0);
    }

    // ========== Scope Nesting ==========

    #[test]
    fn test_nested_scopes() {
        let mut stack = ScopeStack::new();

        // Global -> Module -> Function -> Block
        stack.enter_scope(ScopeKind::Module);
        assert_eq!(stack.depth(), 1);

        stack.enter_scope(ScopeKind::Function);
        assert_eq!(stack.depth(), 2);

        stack.enter_scope(ScopeKind::Block);
        assert_eq!(stack.depth(), 3);

        // Exit back to global
        stack.exit_scope(); // Block
        stack.exit_scope(); // Function
        stack.exit_scope(); // Module
        assert_eq!(stack.depth(), 0);
    }

    #[test]
    fn test_scope_hierarchy() {
        let mut stack = ScopeStack::new();

        stack.enter_scope(ScopeKind::Module);
        let module_idx = stack.current_scope_index();
        assert_eq!(stack.current_scope().parent, Some(0)); // Parent is global

        stack.enter_scope(ScopeKind::Function);
        assert_eq!(stack.current_scope().parent, Some(module_idx));
    }

    #[test]
    fn test_multiple_siblings() {
        let mut stack = ScopeStack::new();

        // Enter first child
        stack.enter_scope(ScopeKind::Function);
        let first_idx = stack.current_scope_index();

        // Exit and enter second child
        stack.exit_scope();
        stack.enter_scope(ScopeKind::Function);
        let second_idx = stack.current_scope_index();

        // Both should have same parent but different indices
        assert_ne!(first_idx, second_idx);
        assert_eq!(stack.current_scope().parent, Some(0));
    }

    // ========== Symbol Resolution ==========

    #[test]
    fn test_lookup_in_current_scope() {
        let mut stack = ScopeStack::new();

        let symbol = test_symbol("x");
        stack.insert(symbol.clone());

        let found = stack.lookup("x");
        assert!(found.is_some());
        assert_eq!(found.unwrap(), &symbol);
    }

    #[test]
    fn test_lookup_in_parent_scope() {
        let mut stack = ScopeStack::new();

        // Insert in global scope
        let symbol = test_symbol("x");
        stack.insert(symbol.clone());

        // Enter function scope
        stack.enter_scope(ScopeKind::Function);

        // Should find symbol from parent (global) scope
        let found = stack.lookup("x");
        assert!(found.is_some());
        assert_eq!(found.unwrap(), &symbol);
    }

    #[test]
    fn test_lookup_through_chain() {
        let mut stack = ScopeStack::new();

        // Insert in global scope
        let symbol = test_symbol("global_var");
        stack.insert(symbol.clone());

        // Create deep nesting
        stack.enter_scope(ScopeKind::Module);
        stack.enter_scope(ScopeKind::Function);
        stack.enter_scope(ScopeKind::Block);

        // Should find symbol from global scope
        let found = stack.lookup("global_var");
        assert!(found.is_some());
        assert_eq!(found.unwrap(), &symbol);
    }

    #[test]
    fn test_lookup_not_found() {
        let stack = ScopeStack::new();
        let found = stack.lookup("nonexistent");
        assert!(found.is_none());
    }

    // ========== Variable Shadowing ==========

    #[test]
    fn test_shadowing_inner_scope() {
        let mut stack = ScopeStack::new();

        // Outer variable
        let outer = Symbol::new(
            "x".to_string(),
            Some("Number".to_string()),
            SymbolKind::Variable,
        );
        stack.insert(outer.clone());

        // Enter inner scope
        stack.enter_scope(ScopeKind::Function);

        // Shadow with different type
        let inner = Symbol::new(
            "x".to_string(),
            Some("String".to_string()),
            SymbolKind::Variable,
        );
        stack.insert(inner.clone());

        // Lookup should find inner (shadowing)
        let found = stack.lookup("x");
        assert!(found.is_some());
        assert_eq!(found.unwrap().type_name, Some("String".to_string()));

        // Exit scope - should see outer again
        stack.exit_scope();
        let found = stack.lookup("x");
        assert!(found.is_some());
        assert_eq!(found.unwrap().type_name, Some("Number".to_string()));
    }

    #[test]
    fn test_shadowing_multiple_levels() {
        let mut stack = ScopeStack::new();

        // Global: x as Number
        let global_x = Symbol::new(
            "x".to_string(),
            Some("Number".to_string()),
            SymbolKind::Variable,
        );
        stack.insert(global_x);

        // Function: x as String
        stack.enter_scope(ScopeKind::Function);
        let func_x = Symbol::new(
            "x".to_string(),
            Some("String".to_string()),
            SymbolKind::Variable,
        );
        stack.insert(func_x);

        // Block: x as Bool
        stack.enter_scope(ScopeKind::Block);
        let block_x = Symbol::new(
            "x".to_string(),
            Some("Bool".to_string()),
            SymbolKind::Variable,
        );
        stack.insert(block_x);

        // Should find Bool (innermost)
        assert_eq!(
            stack.lookup("x").unwrap().type_name,
            Some("Bool".to_string())
        );

        // Exit block - should find String
        stack.exit_scope();
        assert_eq!(
            stack.lookup("x").unwrap().type_name,
            Some("String".to_string())
        );

        // Exit function - should find Number
        stack.exit_scope();
        assert_eq!(
            stack.lookup("x").unwrap().type_name,
            Some("Number".to_string())
        );
    }

    #[test]
    fn test_no_shadowing_in_same_scope() {
        let mut stack = ScopeStack::new();

        let symbol1 = test_symbol("x");
        assert!(stack.insert(symbol1));

        let symbol2 = test_symbol("x");
        assert!(!stack.insert(symbol2)); // Should fail - already exists
    }

    // ========== Edge Cases ==========

    #[test]
    #[should_panic(expected = "Cannot exit global scope")]
    fn test_cannot_exit_global_scope() {
        let mut stack = ScopeStack::new();
        stack.exit_scope(); // Should panic
    }

    #[test]
    fn test_scope_isolation() {
        let mut stack = ScopeStack::new();

        // Enter first child scope
        stack.enter_scope(ScopeKind::Function);
        let symbol_a = test_symbol("a");
        stack.insert(symbol_a);

        // Exit and enter sibling scope
        stack.exit_scope();
        stack.enter_scope(ScopeKind::Function);
        let symbol_b = test_symbol("b");
        stack.insert(symbol_b);

        // Should not find 'a' (in sibling)
        assert!(stack.lookup("a").is_none());
        // Should find 'b' (in current)
        assert!(stack.lookup("b").is_some());
    }

    // ========== Semantic Analyzer Tests ==========

    use crate::lexer::lex;
    use crate::parser::parse;

    // Helper function
    fn analyze_source(source: &str) -> Result<crate::ast::Ast, Vec<SemanticError>> {
        let limits = crate::limits::CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let analyzer = SemanticAnalyzer::new(ast);
        analyzer.analyze()
    }

    #[test]
    fn test_empty_program() {
        let result = analyze_source("");
        assert!(result.is_ok(), "Empty program should analyze successfully");
    }

    #[test]
    fn test_analyzer_initialization() {
        let limits = crate::limits::CompilerLimits::default();
        let tokens = lex("", &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let analyzer = SemanticAnalyzer::new(ast);

        // Should initialize with global scope
        assert_eq!(analyzer.scopes.depth(), 0);
        assert!(analyzer.errors.is_empty());
    }

    #[test]
    fn test_simple_program_with_declarations() {
        // Test that analyzer can traverse a simple program without crashing
        let source = r#"
            x Number: 42
            foo: () { }
        "#;

        let result = analyze_source(source);
        // Should succeed with semantic analysis implemented
        assert!(result.is_ok());
    }
}
