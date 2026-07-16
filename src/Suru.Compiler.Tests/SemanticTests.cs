using Suru.Compiler.Lexing;
using Suru.Compiler.Parsing;
using Suru.Compiler.Semantics;

namespace Suru.Compiler.Tests;

/// <summary>
/// Tests for the <see cref="SemanticAnalyzer"/> — the third pass of the pipeline. Each test
/// lexes and parses a source string first (asserting both were clean), then analyses the
/// resulting <see cref="SuruProgram"/>, so these exercise the analyser against real upstream
/// output the way the pipeline wires it.
/// </summary>
public class SemanticTests
{
    private const string ExampleProgram =
        "extern fn exit(i32)\n" +
        "\n" +
        "fn main() {\n" +
        "    exit(1 + 2 * 3)\n" +
        "}\n";

    /// <summary>Lex, parse, and analyse, asserting the source lexed and parsed cleanly first.</summary>
    private static (SuruProgram Program, SemanticResult Result) Analyze(string source)
    {
        LexResult lexed = Lexer.Tokenize(source);
        Assert.Empty(lexed.Diagnostics);
        ParseResult parsed = Parser.Parse(lexed.Tokens);
        Assert.Empty(parsed.Diagnostics);
        Assert.NotNull(parsed.Program);
        SemanticResult result = SemanticAnalyzer.Analyze(parsed.Program!);
        return (parsed.Program!, result);
    }

    /// <summary>The program passes analysis with no diagnostics.</summary>
    [Fact]
    public void Analyze_ExampleProgram_Succeeds()
    {
        (_, SemanticResult result) = Analyze(ExampleProgram);

        Assert.Empty(result.Diagnostics);
        Assert.True(result.Succeeded);
    }

    /// <summary>
    /// The pass annotates the value tree in place: every node of the argument expression
    /// carries <see cref="SuruType.Int32"/> after analysis (the reason AST nodes are mutable).
    /// </summary>
    [Fact]
    public void Analyze_WritesInt32OntoEveryValueNode()
    {
        (SuruProgram program, _) = Analyze(ExampleProgram);

        CallExpression call = SingleCall(program);
        // exit(1 + 2 * 3) -> ((1 + 2) * 3): the whole tree resolves to i32.
        BinaryExpression outer = Assert.IsType<BinaryExpression>(Assert.Single(call.Arguments));
        Assert.Same(SuruType.Int32, outer.ResolvedType);

        BinaryExpression inner = Assert.IsType<BinaryExpression>(outer.Left);
        Assert.Same(SuruType.Int32, inner.ResolvedType);
        Assert.Same(SuruType.Int32, Assert.IsType<IntLiteral>(inner.Left).ResolvedType);
        Assert.Same(SuruType.Int32, Assert.IsType<IntLiteral>(inner.Right).ResolvedType);
        Assert.Same(SuruType.Int32, Assert.IsType<IntLiteral>(outer.Right).ResolvedType);
    }

    /// <summary>A bare integer argument is typed too.</summary>
    [Fact]
    public void Analyze_BareLiteralArgument_TypedInt32()
    {
        (SuruProgram program, SemanticResult result) =
            Analyze("extern fn exit(i32)\nfn main() { exit(9) }\n");

        Assert.Empty(result.Diagnostics);
        Assert.Same(SuruType.Int32, Assert.Single(SingleCall(program).Arguments).ResolvedType);
    }

    /// <summary>
    /// A callee with no matching <c>extern</c> is a located error naming the callee — not a
    /// crash. The diagnostic points at the callee name and suggests declaring it.
    /// </summary>
    [Fact]
    public void Analyze_UnknownCallee_ReportsLocatedError()
    {
        (_, SemanticResult result) = Analyze("extern fn exit(i32)\nfn main() { print(9) }\n");

        Diagnostic diag = Assert.Single(result.Diagnostics);
        Assert.Equal(
            "call to undeclared function 'print'; declare it with an 'extern fn' declaration",
            diag.Message);
        // Located: points at the callee 'print', not line 0.
        Assert.NotEqual(0, diag.Span.Line);
    }

    /// <summary>A program that declares no externs at all reports its call as undeclared.</summary>
    [Fact]
    public void Analyze_NoExterns_ReportsCallUndeclared()
    {
        (_, SemanticResult result) = Analyze("fn main() { exit(9) }\n");

        Diagnostic diag = Assert.Single(result.Diagnostics);
        Assert.Contains("'exit'", diag.Message);
    }

    /// <summary>
    /// With several externs declared, a callee resolving to any one of them passes — the
    /// resolution is set membership, not position.
    /// </summary>
    [Fact]
    public void Analyze_CalleeAmongMultipleExterns_Succeeds()
    {
        (_, SemanticResult result) = Analyze(
            "extern fn write(i32)\nextern fn exit(i32)\nextern fn read(i32)\nfn main() { exit(0) }\n");

        Assert.Empty(result.Diagnostics);
    }

    /// <summary>
    /// The callee identifier is a function reference, not an integer value, so it is left
    /// untyped — "everything is i32" covers values, not names.
    /// </summary>
    [Fact]
    public void Analyze_CalleeIdentifier_LeftUntyped()
    {
        (SuruProgram program, _) = Analyze(ExampleProgram);

        Assert.Null(SingleCall(program).Callee.ResolvedType);
    }

    /// <summary>
    /// <c>void</c> is a valid return type but not a value type, so declaring it as a
    /// parameter is a located error.
    /// </summary>
    [Fact]
    public void Analyze_VoidParameterType_ReportsLocatedError()
    {
        (_, SemanticResult result) = Analyze("extern fn f(void)\nfn main() {}\n");

        Diagnostic diag = Assert.Single(result.Diagnostics);
        Assert.Equal("'void' is not a value type and cannot be a parameter type", diag.Message);
        Assert.NotEqual(0, diag.Span.Line);
    }

    /// <summary>Passing more arguments than the signature declares is an arity error.</summary>
    [Fact]
    public void Analyze_TooManyArguments_ReportsArityError()
    {
        (_, SemanticResult result) = Analyze("extern fn f(i32)\nfn main() { f(1, 2) }\n");

        Diagnostic diag = Assert.Single(result.Diagnostics);
        Assert.Equal("function 'f' expects 1 argument(s), but 2 were provided", diag.Message);
    }

    /// <summary>Passing fewer arguments than the signature declares is an arity error.</summary>
    [Fact]
    public void Analyze_TooFewArguments_ReportsArityError()
    {
        (_, SemanticResult result) = Analyze("extern fn f(i32, i32)\nfn main() { f(1) }\n");

        Diagnostic diag = Assert.Single(result.Diagnostics);
        Assert.Equal("function 'f' expects 2 argument(s), but 1 were provided", diag.Message);
    }

    /// <summary>A zero-parameter extern called with no arguments passes.</summary>
    [Fact]
    public void Analyze_ZeroParameterExtern_CalledWithNoArgs_Succeeds()
    {
        (_, SemanticResult result) = Analyze("extern fn flush()\nfn main() { flush() }\n");

        Assert.Empty(result.Diagnostics);
        Assert.True(result.Succeeded);
    }

    /// <summary>A call is stamped with the extern's declared value return type.</summary>
    [Fact]
    public void Analyze_Call_ResolvesToExternReturnType_Int32()
    {
        (SuruProgram program, SemanticResult result) =
            Analyze("extern fn read(i32) i32\nfn main() { read(0) }\n");

        Assert.Empty(result.Diagnostics);
        Assert.Same(SuruType.Int32, SingleCall(program).ResolvedType);
    }

    /// <summary>A call to a void extern is stamped <c>void</c>, not left untyped.</summary>
    [Fact]
    public void Analyze_Call_ResolvesToExternReturnType_Void()
    {
        (SuruProgram program, _) = Analyze("extern fn exit(i32)\nfn main() { exit(0) }\n");

        Assert.Same(SuruType.Void, SingleCall(program).ResolvedType);
    }

    /// <summary>Null program is a caller error, guarded like the rest of the public API.</summary>
    [Fact]
    public void Analyze_NullProgram_Throws()
    {
        Assert.Throws<ArgumentNullException>(() => SemanticAnalyzer.Analyze(null!));
    }

    // --- Helpers ---

    /// <summary>The single call in <c>main</c>'s body.</summary>
    private static CallExpression SingleCall(SuruProgram program)
    {
        Statement stmt = Assert.Single(program.Main.Body.Statements);
        return Assert.IsType<CallExpression>(Assert.IsType<ExpressionStatement>(stmt).Expression);
    }
}
