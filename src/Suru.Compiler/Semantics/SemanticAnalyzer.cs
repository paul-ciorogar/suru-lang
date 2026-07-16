using Suru.Compiler.Parsing;

namespace Suru.Compiler.Semantics;

/// <summary>
/// The outcome of a <see cref="SemanticAnalyzer.Analyze"/> call: the problems found while
/// checking the program.
///
/// Unlike <see cref="ParseResult"/>, the semantic pass produces no new tree — it annotates
/// the parser's <see cref="SuruProgram"/> in place (that is why AST nodes are mutable), so
/// the only result to carry back is the diagnostic list. <see cref="Diagnostics"/> is empty
/// when the program is well-formed; <see cref="Succeeded"/> is the convenience predicate the
/// pipeline branches on.
/// </summary>
/// <param name="Diagnostics">Problems found while analysing; empty on a valid program.</param>
public readonly record struct SemanticResult(IReadOnlyList<Diagnostic> Diagnostics)
{
    /// <summary>True when no problems were found.</summary>
    public bool Succeeded => Diagnostics.Count == 0;
}

/// <summary>
/// The semantic pass: the third stage of the pipeline, run on the parser's
/// <see cref="SuruProgram"/> before codegen.
///
/// Like the earlier passes it never throws for a bad <em>program</em> — it records located
/// <see cref="Diagnostic"/>s and keeps going (only <see cref="ArgumentNullException"/> guards
/// a null program, a caller error).
/// </summary>
public static class SemanticAnalyzer
{
    /// <summary>
    /// Analyse a parsed program, annotating its nodes with resolved types and collecting any
    /// problems found.
    /// </summary>
    /// <param name="program">The program to analyse (its nodes are annotated in place).</param>
    /// <returns>A <see cref="SemanticResult"/> listing every problem found.</returns>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="program"/> is null.</exception>
    public static SemanticResult Analyze(SuruProgram program)
    {
        ArgumentNullException.ThrowIfNull(program);

        // A single Analysis owns the mutable diagnostic list + resolved extern set for
        // this call, keeping Analyze trivially re-entrant.
        Analysis analysis = new(program);
        return analysis.Run();
    }

    /// <summary>
    /// The mutable state for one <see cref="Analyze"/> call: the declared extern names a
    /// callee may resolve to, plus the growing diagnostic list.
    /// </summary>
    private sealed class Analysis(SuruProgram program)
    {
        private readonly SuruProgram _program = program;
        private readonly List<Diagnostic> _diagnostics = new();

        /// <summary>
        /// The functions brought into scope by <c>extern fn</c> declarations.
        /// </summary>
        private readonly Dictionary<string, ExternDeclaration> _externs =
            new(StringComparer.Ordinal);

        /// <summary>Run the analysis to completion and return the collected result.</summary>
        public SemanticResult Run()
        {
            BuildExterns();
            AnalyzeFunction(_program.Main);
            return new SemanticResult(_diagnostics);
        }

        /// <summary>
        /// Resolve and record every extern signature. 
        /// </summary>
        private void BuildExterns()
        {
            foreach (ExternDeclaration declaration in _program.Externs)
            {
                foreach (TypeReference parameter in declaration.Parameters)
                {
                    ResolveType(parameter, isParameter: true);
                }
                ResolveType(declaration.ReturnType, isParameter: false);

                _externs.TryAdd(declaration.Name, declaration);
            }
        }

        /// <summary>
        /// Resolve a written type reference to its <see cref="SuruType"/>, stamping the node
        /// and reporting a located diagnostic on failure. <c>void</c> is a valid return type
        /// but never a value type, so it is rejected in parameter position.
        /// </summary>
        private void ResolveType(TypeReference reference, bool isParameter)
        {
            if (!SuruType.TryResolve(reference.Name, out SuruType? type))
            {
                // Defensive: only valid type keywords lex as a type name, so an unknown type
                // is normally caught by the parser. This guards a lexer/SuruType drift.
                _diagnostics.Add(Diagnostic.Error(
                    reference.Span, $"unknown type '{reference.Name}'"));
                return;
            }

            if (isParameter && type == SuruType.Void)
            {
                _diagnostics.Add(Diagnostic.Error(
                    reference.Span, "'void' is not a value type and cannot be a parameter type"));
                return;
            }

            reference.ResolvedType = type;
        }

        /// <summary>Analyse one function: every statement in its body, in order.</summary>
        private void AnalyzeFunction(FunctionDefinition function)
        {
            foreach (Statement statement in function.Body.Statements)
            {
                AnalyzeStatement(statement);
            }
        }

        /// <summary>
        /// Analyse one statement. An unexpected statement kind is reported defensively rather
        /// than crashing the pass.
        /// </summary>
        private void AnalyzeStatement(Statement statement)
        {
            if (statement is ExpressionStatement expressionStatement)
            {
                AnalyzeStatementExpression(expressionStatement.Expression);
                return;
            }

            _diagnostics.Add(Diagnostic.Error(
                statement.Span, "unsupported statement"));
        }

        /// <summary>
        /// Analyse the expression in statement position. 
        /// </summary>
        private void AnalyzeStatementExpression(Expression expression)
        {
            if (expression is CallExpression call)
            {
                AnalyzeCall(call);
                return;
            }

            _diagnostics.Add(Diagnostic.Error(
                expression.Span, "expected a call"));
        }

        private void AnalyzeCall(CallExpression call)
        {
            string callee = call.Callee.Name;

            // Type every argument regardless of whether the callee resolves, so the value
            // tree is annotated and any typing problem inside it is still reported.
            foreach (Expression argument in call.Arguments)
            {
                InferType(argument);
            }

            if (!_externs.TryGetValue(callee, out ExternDeclaration? declaration))
            {
                _diagnostics.Add(Diagnostic.Error(
                    call.Callee.Span,
                    $"call to undeclared function '{callee}'; declare it with an 'extern fn' declaration"));
                return;
            }

            // Arity: the call must pass exactly as many arguments as the signature declares.
            // (Per-argument value-type comparison waits until InferType can produce more than
            // i32 — with one value type no well-formed call can mismatch.)
            if (call.Arguments.Count != declaration.Parameters.Count)
            {
                _diagnostics.Add(Diagnostic.Error(
                    call.Span,
                    $"function '{callee}' expects {declaration.Parameters.Count} argument(s), " +
                    $"but {call.Arguments.Count} were provided"));
            }

            // The call yields whatever the extern returns (i32, void, …). Null only if the
            // return type itself failed to resolve (the defensive case above).
            call.ResolvedType = declaration.ReturnType.ResolvedType;
        }

        /// <summary>
        /// Infer and record the type of a value expression, returning it. 
        /// </summary>
        private SuruType InferType(Expression expression)
        {
            switch (expression)
            {
                case IntLiteral literal:
                    literal.ResolvedType = SuruType.Int32;
                    return SuruType.Int32;

                case BinaryExpression binary:
                    // Both operands must be integers.
                    RequireInt(binary.Left);
                    RequireInt(binary.Right);
                    binary.ResolvedType = SuruType.Int32;
                    return SuruType.Int32;

                default:
                    // No other expression can appear as a value.
                    _diagnostics.Add(Diagnostic.Error(
                        expression.Span, "expected an integer expression"));
                    expression.ResolvedType = SuruType.Int32;
                    return SuruType.Int32;
            }
        }

        /// <summary>
        /// Infer an operand's type and report if it is not an integer. Kept separate from
        /// <see cref="InferType"/> so the check reads at the call site and grows naturally
        /// once more types than <see cref="SuruType.Int32"/> exist.
        /// </summary>
        private void RequireInt(Expression operand)
        {
            SuruType type = InferType(operand);
            if (type != SuruType.Int32)
            {
                _diagnostics.Add(Diagnostic.Error(
                    operand.Span, $"expected an integer operand, found '{type}'"));
            }
        }
    }
}
