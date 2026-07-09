using Suru.Compiler.Lexing;
using Suru.Compiler.Parsing;

namespace Suru.Compiler.Tests;

/// <summary>
/// Tests for the <see cref="Parser"/> — the second pass of the compile pipeline. Each
/// test lexes a source string and parses the resulting token stream, so these exercise
/// the parser against real lexer output (the way the pipeline wires them).
/// </summary>
public class ParserTests
{
    private const string ExampleProgram =
        "extern exit\n" +
        "\n" +
        "fn main() {\n" +
        "    exit(1 + 2 * 3)\n" +
        "}\n";

    /// <summary>Lex then parse, asserting the source lexed cleanly first.</summary>
    private static ParseResult Parse(string source)
    {
        LexResult lexed = Lexer.Tokenize(source);
        Assert.Empty(lexed.Diagnostics);
        return Parser.Parse(lexed.Tokens);
    }

    /// <summary>Assert the result carries a program and return it (xUnit's Assert.NotNull is void).</summary>
    private static SuruProgram AssertProgram(ParseResult result)
    {
        Assert.NotNull(result.Program);
        return result.Program!;
    }

    /// <summary>
    /// The example program parses into the expected AST —
    /// one <c>extern exit</c>, a function <c>main</c> whose body is a single call to
    /// <c>exit</c> — with no diagnostics.
    /// </summary>
    [Fact]
    public void Parse_ExampleProgram_ProducesExpectedAst()
    {
        ParseResult result = Parse(ExampleProgram);

        Assert.Empty(result.Diagnostics);
        SuruProgram program = AssertProgram(result);

        ExternDeclaration ext = Assert.Single(program.Externs);
        Assert.Equal("exit", ext.Name);

        Assert.Equal("main", program.Main.Name);
        Statement stmt = Assert.Single(program.Main.Body.Statements);
        ExpressionStatement exprStmt = Assert.IsType<ExpressionStatement>(stmt);
        CallExpression call = Assert.IsType<CallExpression>(exprStmt.Expression);
        Assert.Equal("exit", call.Callee.Name);
    }

    /// <summary>
    /// The headline language rule: with no operator precedence, <c>1 + 2 * 3</c> nests
    /// strictly left-to-right as <c>((1 + 2) * 3)</c> — the outer node multiplies, and its
    /// left operand is the <c>1 + 2</c> addition. (This is what makes the example exit 9,
    /// not 7.)
    /// </summary>
    [Fact]
    public void Parse_Arithmetic_NestsLeftToRightWithNoPrecedence()
    {
        BinaryExpression outer = Assert.IsType<BinaryExpression>(ParseArgumentOf("exit(1 + 2 * 3)"));

        // Outermost operation is the LAST one written: the multiply.
        Assert.Equal(BinaryOperator.Multiply, outer.Operator);
        Assert.Equal(3, Assert.IsType<IntLiteral>(outer.Right).Value);

        // Its left operand is the (1 + 2) addition.
        BinaryExpression inner = Assert.IsType<BinaryExpression>(outer.Left);
        Assert.Equal(BinaryOperator.Add, inner.Operator);
        Assert.Equal(1, Assert.IsType<IntLiteral>(inner.Left).Value);
        Assert.Equal(2, Assert.IsType<IntLiteral>(inner.Right).Value);
    }

    /// <summary>
    /// Parentheses are the only way to override the flat grouping: <c>1 + (2 * 3)</c>
    /// makes addition the outer node with the multiplication on its right.
    /// </summary>
    [Fact]
    public void Parse_Parentheses_RegroupTheTree()
    {
        BinaryExpression outer = Assert.IsType<BinaryExpression>(ParseArgumentOf("f(1 + (2 * 3))"));

        Assert.Equal(BinaryOperator.Add, outer.Operator);
        Assert.Equal(1, Assert.IsType<IntLiteral>(outer.Left).Value);

        BinaryExpression grouped = Assert.IsType<BinaryExpression>(outer.Right);
        Assert.Equal(BinaryOperator.Multiply, grouped.Operator);
        Assert.Equal(2, Assert.IsType<IntLiteral>(grouped.Left).Value);
        Assert.Equal(3, Assert.IsType<IntLiteral>(grouped.Right).Value);
    }

    /// <summary>Each operator token maps to its <see cref="BinaryOperator"/>.</summary>
    [Theory]
    [InlineData("f(1 + 2)", BinaryOperator.Add)]
    [InlineData("f(1 - 2)", BinaryOperator.Subtract)]
    [InlineData("f(1 * 2)", BinaryOperator.Multiply)]
    [InlineData("f(1 / 2)", BinaryOperator.Divide)]
    public void Parse_Operator_MapsToBinaryOperator(string call, BinaryOperator expected)
    {
        BinaryExpression expr = Assert.IsType<BinaryExpression>(ParseArgumentOf(call));
        Assert.Equal(expected, expr.Operator);
    }

    /// <summary>A bare integer argument parses to an <see cref="IntLiteral"/> with its value.</summary>
    [Fact]
    public void Parse_MultiDigitLiteral_CapturesValue()
    {
        IntLiteral literal = Assert.IsType<IntLiteral>(ParseArgumentOf("f(255)"));
        Assert.Equal(255, literal.Value);
    }

    /// <summary>Zero, one, or many <c>extern</c> declarations all parse, in source order.</summary>
    [Theory]
    [InlineData("fn main() { go(1) }", 0)]
    [InlineData("extern a\nfn main() { go(1) }", 1)]
    [InlineData("extern a\nextern b\nextern c\nfn main() { go(1) }", 3)]
    public void Parse_ExternCount_CollectsAllInOrder(string source, int expectedCount)
    {
        ParseResult result = Parse(source);

        Assert.Empty(result.Diagnostics);
        SuruProgram program = AssertProgram(result);
        Assert.Equal(expectedCount, program.Externs.Count);
    }

    /// <summary>
    /// The call node spans from the callee through the closing paren, so a later pass can
    /// point a diagnostic at the whole call.
    /// </summary>
    [Fact]
    public void Parse_Call_SpansTheWholeCall()
    {
        CallExpression call = Assert.IsType<CallExpression>(
            ((ExpressionStatement)ParseSingleStatement("fn main() { exit(9) }")).Expression);

        // "exit(9)" starts at offset 12 ("fn main() { " is 12 chars) and is 7 chars long.
        Assert.Equal(12, call.Span.Offset);
        Assert.Equal(7, call.Span.Length);
    }

    /// <summary>
    /// A program may define any number of functions: all are collected in source order,
    /// and the one named <c>main</c> is surfaced as the entry point regardless of position.
    /// </summary>
    [Fact]
    public void Parse_MultipleFunctions_AllCollectedAndMainResolved()
    {
        ParseResult result = Parse(
            "fn helper() { go(1) }\n" +
            "fn main() { exit(2) }\n" +
            "fn other() { go(3) }\n");

        Assert.Empty(result.Diagnostics);
        SuruProgram program = AssertProgram(result);

        Assert.Equal(3, program.Functions.Count);
        Assert.Equal(new[] { "helper", "main", "other" }, program.Functions.Select(f => f.Name));
        // The entry point is the one named main, not merely the first defined.
        Assert.Equal("main", program.Main.Name);
    }

    // --- Error recovery ---

    /// <summary>
    /// A program with no function definition is a located error and yields no program —
    /// but is reported, not thrown.
    /// </summary>
    [Fact]
    public void Parse_NoFunction_ReportsMissingFunctionDefinition()
    {
        ParseResult result = Parse("extern exit\n");

        Assert.Null(result.Program);
        Diagnostic diag = Assert.Single(result.Diagnostics);
        Assert.Equal("a program must define a 'main' function", diag.Message);
    }

    /// <summary>
    /// Functions defined but none named <c>main</c> is a located error with no recovered
    /// program: there is nothing to enter.
    /// </summary>
    [Fact]
    public void Parse_NoMain_ReportsMissingEntryPoint()
    {
        ParseResult result = Parse("fn helper() { go(1) }\n");

        Assert.Null(result.Program);
        Diagnostic diag = Assert.Single(result.Diagnostics);
        Assert.Equal("a program must define a 'main' function", diag.Message);
    }

    /// <summary>
    /// Two functions named <c>main</c> is a located error pointing at the duplicate; the
    /// first definition is kept as the entry point so the rest of the pipeline can proceed.
    /// </summary>
    [Fact]
    public void Parse_DuplicateMain_ReportsErrorAndKeepsFirst()
    {
        ParseResult result = Parse(
            "fn main() { exit(1) }\n" +
            "fn main() { exit(2) }\n");

        Diagnostic diag = Assert.Single(result.Diagnostics);
        Assert.Equal("a program may define only one 'main' function", diag.Message);
        // Both definitions are collected, and the first main is the recovered entry point.
        SuruProgram program = AssertProgram(result);
        Assert.Equal(2, program.Functions.Count);
        Assert.Same(program.Functions[0], program.Main);
    }

    /// <summary>An unclosed call reports a located diagnostic rather than throwing.</summary>
    [Fact]
    public void Parse_MissingCloseParen_ReportsLocatedError()
    {
        ParseResult result = Parse("fn main() { exit(9 }");

        Assert.NotEmpty(result.Diagnostics);
        Assert.All(result.Diagnostics, d => Assert.NotEqual(0, d.Span.Line));
    }

    /// <summary>An <c>extern</c> with no name reports a located diagnostic.</summary>
    [Fact]
    public void Parse_ExternWithoutName_ReportsLocatedError()
    {
        ParseResult result = Parse("extern\nfn main() { go(1) }");

        Diagnostic diag = Assert.Single(result.Diagnostics);
        Assert.Equal("expected an identifier after 'extern'", diag.Message);
    }

    /// <summary>A non-expression token where an argument belongs is a located error.</summary>
    [Fact]
    public void Parse_NonExpressionArgument_ReportsLocatedError()
    {
        ParseResult result = Parse("fn main() { exit(*) }");

        Diagnostic diag = Assert.Single(result.Diagnostics);
        Assert.Equal("expected an expression", diag.Message);
    }

    /// <summary>
    /// The headline of the reworked error handling: a file with two independent mistakes —
    /// a malformed <c>extern</c> AND a malformed call — reports BOTH in a single pass, and
    /// still recovers the valid parts around them. Panic-mode synchronisation resumes at
    /// each declaration/statement boundary instead of aborting at the first error.
    /// </summary>
    [Fact]
    public void Parse_MultipleIndependentErrors_AllReportedInOnePass()
    {
        // First extern has no name; the call in main has no closing paren. Two distinct,
        // independently-recoverable mistakes.
        string source =
            "extern\n" +
            "extern write\n" +
            "fn main() { exit(9 }\n";

        ParseResult result = Parse(source);

        // Both problems surface — synchronisation kept going past the first.
        Assert.True(result.Diagnostics.Count >= 2,
            $"expected at least 2 diagnostics, got {result.Diagnostics.Count}");
        // The valid 'extern write' between/after the errors was still recovered.
        SuruProgram program = AssertProgram(result);
        Assert.Contains(program.Externs, e => e.Name == "write");
        Assert.Equal("main", program.Main.Name);
    }

    /// <summary>Null tokens is a caller error, guarded like the rest of the public API.</summary>
    [Fact]
    public void Parse_NullTokens_Throws()
    {
        Assert.Throws<ArgumentNullException>(() => Parser.Parse(null!));
    }

    // --- Helpers ---

    /// <summary>Parse a whole program and return its single body statement.</summary>
    private static Statement ParseSingleStatement(string source)
    {
        ParseResult result = Parse(source);
        Assert.Empty(result.Diagnostics);
        SuruProgram program = AssertProgram(result);
        return Assert.Single(program.Main.Body.Statements);
    }

    /// <summary>
    /// Parse a program whose <c>main</c> body is exactly the given call, and return that
    /// call's single argument expression — the unit these arithmetic tests care about.
    /// </summary>
    private static Expression ParseArgumentOf(string call)
    {
        Statement stmt = ParseSingleStatement($"fn main() {{ {call} }}");
        CallExpression callExpr = Assert.IsType<CallExpression>(
            Assert.IsType<ExpressionStatement>(stmt).Expression);
        return callExpr.Argument;
    }
}
