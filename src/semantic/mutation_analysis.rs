// Mutation analysis for the lowering pass
//
// This module computes which parameters are mutated by each function.
// The results are consumed by the lowering pass (src/lower/) to decide
// whether heap-typed arguments should be passed ByRef or ByOwnership.
//
// A parameter is mutated if the function body:
//   1. Directly assigns a field on it:  `param.field: value`
//   2. Calls a method on it that mutates `this`:  `param.grow()`
//   3. Passes it to a callee that mutates that argument position (transitive)
//
// Mutation flags are stored as a u64 bitmask, limiting functions to 64 parameters.
// Bit i is set if declared parameter i (0-indexed) is mutated.

use std::collections::HashMap;

use crate::ast::{Ast, NodeType};

use super::{Type, TypeId, TypeRegistry};

// ========== Public types ==========

/// Information about a function declaration collected during semantic traversal.
/// Stored by `visit_function_decl` and consumed by `compute_all_mutations`
/// (which runs after type unification when all types are fully resolved).
pub(super) struct FunctionDeclInfo {
    /// The function's declared name.
    pub name: String,
    /// Declared parameter names, in order.
    pub param_names: Vec<String>,
    /// TypeIds for each parameter (may be `Type::Unknown` for untyped params).
    /// Resolved through the substitution in `compute_all_mutations`.
    pub param_type_ids: Vec<TypeId>,
    /// AST index of the function body block, if present.
    pub body_idx: Option<usize>,
    /// The struct TypeId in scope when this function was visited, if any.
    /// Set for struct methods; `None` for top-level functions.
    pub struct_context: Option<TypeId>,
}

/// Output of the per-function mutation walk.
pub(super) struct MutationResult {
    /// Bitmask: bit i is set if declared parameter i is mutated within the body.
    /// Functions are limited to 64 parameters by this representation.
    pub param_mutations: u64,
    /// True if the body contains any `this.field: value` assignment.
    /// Relevant only for struct methods.
    pub mutates_this: bool,
}

// ========== Public entry point ==========

/// Walk the function body at `body_idx` and compute mutation flags.
///
/// `param_names` and `param_types` must be in declaration order and
/// have the same length.  `param_types` should already be substitution-
/// resolved (concrete types, not TypeVars) so that method-mutation
/// lookups work correctly.
///
/// `function_mutations` and `method_this_mutations` hold the results of
/// functions/methods already processed (in AST traversal order), enabling
/// transitive mutation detection for callees defined earlier.
pub(super) fn compute_param_mutations(
    ast: &Ast,
    body_idx: usize,
    param_names: &[String],
    param_types: &[TypeId],
    type_registry: &TypeRegistry,
    function_mutations: &HashMap<usize, u64>,
    function_node_by_name: &HashMap<String, usize>,
    method_this_mutations: &HashMap<(TypeId, String), bool>,
) -> MutationResult {
    let mut result = MutationResult { param_mutations: 0, mutates_this: false };
    walk_node(
        ast,
        body_idx,
        param_names,
        param_types,
        type_registry,
        function_mutations,
        function_node_by_name,
        method_this_mutations,
        &mut result,
    );
    result
}

// ========== Private walk helpers ==========

fn walk_node(
    ast: &Ast,
    node_idx: usize,
    param_names: &[String],
    param_types: &[TypeId],
    type_registry: &TypeRegistry,
    function_mutations: &HashMap<usize, u64>,
    function_node_by_name: &HashMap<String, usize>,
    method_this_mutations: &HashMap<(TypeId, String), bool>,
    result: &mut MutationResult,
) {
    let node_type = ast.nodes[node_idx].node_type;

    // Never recurse into nested function declarations: they have independent
    // mutation tracking and their mutations don't affect the outer function's params.
    if node_type == NodeType::FunctionDecl {
        return;
    }

    match node_type {
        NodeType::PropertyAssignment => {
            check_property_assignment(ast, node_idx, param_names, result);
        }
        NodeType::MethodCall => {
            check_method_call(
                ast,
                node_idx,
                param_names,
                param_types,
                type_registry,
                method_this_mutations,
                result,
            );
        }
        NodeType::FunctionCall => {
            check_function_call(
                ast,
                node_idx,
                param_names,
                function_mutations,
                function_node_by_name,
                result,
            );
        }
        _ => {}
    }

    // Recurse into all children.
    for child_idx in ast.children(node_idx) {
        walk_node(
            ast,
            child_idx,
            param_names,
            param_types,
            type_registry,
            function_mutations,
            function_node_by_name,
            method_this_mutations,
            result,
        );
    }
}

/// Detects direct field mutation: `param.field: value` or `this.field: value`.
///
/// AST shape:
/// ```text
/// PropertyAssignment
///   PropertyAccess           ← LHS
///     <receiver>             ← This or Identifier
///     Identifier 'fieldName'
///   <value expr>             ← RHS
/// ```
fn check_property_assignment(
    ast: &Ast,
    node_idx: usize,
    param_names: &[String],
    result: &mut MutationResult,
) {
    let Some(lhs_idx) = ast.nodes[node_idx].first_child else { return };
    // lhs_idx is a PropertyAccess node; its first child is the receiver.
    let Some(receiver_idx) = ast.nodes[lhs_idx].first_child else { return };

    match ast.nodes[receiver_idx].node_type {
        NodeType::This => {
            result.mutates_this = true;
        }
        NodeType::Identifier => {
            if let Some(name) = ast.node_text(receiver_idx) {
                if let Some(idx) = param_names.iter().position(|p| p == name) {
                    if idx < 64 {
                        result.param_mutations |= 1u64 << idx;
                    }
                }
            }
        }
        _ => {}
    }
}

/// Detects mutation via a method call: `param.method()` where the method
/// is known to mutate `this`.
///
/// AST shape:
/// ```text
/// MethodCall
///   <receiver>   ← Identifier (first child)
///   Identifier   ← method name (second child)
///   ArgList      (third child)
/// ```
///
/// Lookup strategy:
/// - If the param's resolved type is a known `Struct`, do a precise
///   `(struct_type_id, method_name)` lookup.
/// - Otherwise (Unknown / TypeVar), fall back to a name-only scan across
///   all known struct methods. This is conservative: it may produce a
///   false positive if two different structs have a method with the same
///   name and one of them mutates `this`, but it never produces false
///   negatives.
fn check_method_call(
    ast: &Ast,
    node_idx: usize,
    param_names: &[String],
    param_types: &[TypeId],
    type_registry: &TypeRegistry,
    method_this_mutations: &HashMap<(TypeId, String), bool>,
    result: &mut MutationResult,
) {
    let Some(receiver_idx) = ast.nodes[node_idx].first_child else { return };

    if ast.nodes[receiver_idx].node_type != NodeType::Identifier {
        return;
    }

    let Some(receiver_name) = ast.node_text(receiver_idx) else { return };
    let Some(param_idx) = param_names.iter().position(|p| p == receiver_name) else { return };

    if param_idx >= 64 {
        return;
    }

    // Get the method name from the second child.
    let Some(method_name_idx) = ast.nodes[receiver_idx].next_sibling else { return };
    let Some(method_name) = ast.node_text(method_name_idx) else { return };

    let is_mutating = if param_idx < param_types.len() {
        let param_type_id = param_types[param_idx];
        let ty = type_registry.resolve(param_type_id);
        if matches!(ty, Type::Struct(_)) {
            // Precise lookup: we know the exact struct type.
            method_this_mutations
                .get(&(param_type_id, method_name.to_string()))
                .copied()
                .unwrap_or(false)
        } else {
            // Conservative fallback: check if any struct method with this
            // name is known to mutate `this`.
            method_this_mutations
                .iter()
                .any(|((_, m), &mutates)| m == method_name && mutates)
        }
    } else {
        false
    };

    if is_mutating {
        result.param_mutations |= 1u64 << param_idx;
    }
}

/// Detects transitive mutation via a function call: `helper(param)` where
/// `helper` is known to mutate its argument at that position.
///
/// AST shape:
/// ```text
/// FunctionCall
///   Identifier   ← function name (first child)
///   ArgList      (second child)
///     <arg1>
///     ...
/// ```
fn check_function_call(
    ast: &Ast,
    node_idx: usize,
    param_names: &[String],
    function_mutations: &HashMap<usize, u64>,
    function_node_by_name: &HashMap<String, usize>,
    result: &mut MutationResult,
) {
    let Some(func_name_idx) = ast.nodes[node_idx].first_child else { return };
    let Some(func_name) = ast.node_text(func_name_idx) else { return };

    let Some(&callee_node_idx) = function_node_by_name.get(func_name) else { return };
    let Some(&callee_mutations) = function_mutations.get(&callee_node_idx) else { return };

    if callee_mutations == 0 {
        return;
    }

    let Some(arg_list_idx) = ast.nodes[func_name_idx].next_sibling else { return };

    for (arg_pos, arg_idx) in ast.children(arg_list_idx).enumerate() {
        if arg_pos >= 64 {
            break;
        }
        if callee_mutations & (1u64 << arg_pos) == 0 {
            continue;
        }

        // This argument position is mutated by the callee — check if the
        // argument is one of our own parameters.
        if ast.nodes[arg_idx].node_type == NodeType::Identifier {
            if let Some(arg_name) = ast.node_text(arg_idx) {
                if let Some(param_idx) = param_names.iter().position(|p| p == arg_name) {
                    if param_idx < 64 {
                        result.param_mutations |= 1u64 << param_idx;
                    }
                }
            }
        }
    }
}

// ========== Tests ==========

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::lexer::lex;
    use crate::limits::CompilerLimits;
    use crate::parser::parse;
    use crate::semantic::SemanticAnalyzer;

    /// Analyzes `source` and returns a map from function name → mutation bitmask.
    /// Panics if analysis fails (semantic errors).
    fn analyze_mutations(source: &str) -> HashMap<String, u64> {
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let analyzer = SemanticAnalyzer::new(ast);
        let output = analyzer
            .analyze_with_types()
            .expect("Analysis should succeed without errors");

        let mut result: HashMap<String, u64> = HashMap::new();
        for (&node_idx, &mask) in &output.function_mutations {
            if let Some(name) = output.ast.function_decl(node_idx).name() {
                result.insert(name.to_string(), mask);
            }
        }
        result
    }

    /// Returns true if bit `param_idx` is set in `mask`.
    fn is_param_mutated(mask: u64, param_idx: usize) -> bool {
        mask & (1u64 << param_idx) != 0
    }

    // ── Test 1: function only reads param fields → no mutations ──────────────

    #[test]
    fn test_no_mutation_reads_only() {
        // `describe` only reads `person.name` via a VarDecl — no assignment.
        let source = r#"
describe: (person) {
  n: person.name
}
"#;
        let mutations = analyze_mutations(source);
        let mask = mutations.get("describe").copied().unwrap_or(0);
        assert_eq!(mask, 0, "Reading a field should not count as mutation");
    }

    // ── Test 2: direct field assignment on param → that param mutated ────────

    #[test]
    fn test_direct_field_assignment_mutates_param() {
        // `rename` assigns `person.name: "Bob"` — first param must be marked.
        let source = r#"
rename: (person) {
  person.name: "Bob"
}
"#;
        let mutations = analyze_mutations(source);
        let mask = mutations.get("rename").copied().unwrap_or(0);
        assert!(
            is_param_mutated(mask, 0),
            "Direct field assignment should mark param 0 as mutated (mask={mask})"
        );
    }

    // ── Test 3: method call that mutates `this` → caller param mutated ───────

    #[test]
    fn test_mutating_method_call_marks_param() {
        // `container.grow()` assigns `this.count` → the method mutates `this`.
        // `update` calls `c.grow()` → param `c` (index 0) must be marked.
        // A call site `update(container)` is included so the type of `c`
        // can be resolved; the conservative name-based fallback also handles
        // the unresolved-type case.
        let source = r#"
container: {
  count: 0
  grow: () {
    this.count: 1
  }
}
update: (c) {
  result: c.grow()
}
update(container)
"#;
        let mutations = analyze_mutations(source);
        let mask = mutations.get("update").copied().unwrap_or(0);
        assert!(
            is_param_mutated(mask, 0),
            "Calling a mutating method on param 0 should mark it as mutated (mask={mask})"
        );
    }

    // ── Test 4: transitive — callee mutates param → caller param mutated ─────

    #[test]
    fn test_transitive_function_call_marks_param() {
        // `setName` directly mutates its first param.
        // `wrapper` calls `setName(person)` → `person` (index 0) is transitively mutated.
        let source = r#"
setName: (p) {
  p.name: "Bob"
}
wrapper: (person) {
  setName(person)
}
"#;
        let mutations = analyze_mutations(source);

        // `setName` must report param 0 as directly mutated.
        let set_name_mask = mutations.get("setName").copied().unwrap_or(0);
        assert!(
            is_param_mutated(set_name_mask, 0),
            "setName should mutate its first param (mask={set_name_mask})"
        );

        // `wrapper` must report param 0 as transitively mutated.
        let wrapper_mask = mutations.get("wrapper").copied().unwrap_or(0);
        assert!(
            is_param_mutated(wrapper_mask, 0),
            "wrapper should transitively mutate its first param via setName (mask={wrapper_mask})"
        );
    }

    // ── Test 5: second param only, first param untouched ─────────────────────

    #[test]
    fn test_only_second_param_mutated() {
        // `swap` assigns `b.value: 0` — only second param (index 1) is mutated.
        let source = r#"
swap: (a, b) {
  b.value: 0
}
"#;
        let mutations = analyze_mutations(source);
        let mask = mutations.get("swap").copied().unwrap_or(0);
        assert!(
            !is_param_mutated(mask, 0),
            "First param 'a' should NOT be mutated (mask={mask})"
        );
        assert!(
            is_param_mutated(mask, 1),
            "Second param 'b' SHOULD be mutated (mask={mask})"
        );
    }
}
