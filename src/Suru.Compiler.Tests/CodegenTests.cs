using Suru.Compiler.CodeGen;
using Suru.Compiler.Lexing;
using Suru.Compiler.Parsing;
using Suru.Compiler.Semantics;

namespace Suru.Compiler.Tests;

/// <summary>
/// Tests for the <see cref="CodeGenerator"/> — the fourth pass of the pipeline. Each test lexes,
/// parses, and analyses a source string first (asserting every upstream stage is clean, the way
/// the pipeline wires them), then generates the LLVM module and asserts on its IR text — the same
/// text the <c>ir</c> subcommand will dump. Exercising the real front-end also drives LLVMSharp's
/// managed wrappers against the live native libLLVM.
/// </summary>
public class CodegenTests
{
    private const string ExampleProgram =
        "extern fn exit(i32)\n" +
        "\n" +
        "fn main() {\n" +
        "    exit(1 + 2 * 3)\n" +
        "}\n";

    /// <summary>Lex, parse, analyse, and generate, asserting the source was clean through sema.</summary>
    private static CodegenResult Generate(string source)
    {
        LexResult lexed = Lexer.Tokenize(source);
        Assert.Empty(lexed.Diagnostics);
        ParseResult parsed = Parser.Parse(lexed.Tokens);
        Assert.Empty(parsed.Diagnostics);
        Assert.NotNull(parsed.Program);
        SemanticResult analyzed = SemanticAnalyzer.Analyze(parsed.Program!);
        Assert.True(analyzed.Succeeded);
        return CodeGenerator.Generate(parsed.Program!);
    }

    /// <summary>
    /// The example program lowers to a module with the extern declare, <c>main</c>, and the call.
    ///
    /// Because every operand is a constant, LLVM's builder constant-folds the whole expression, so
    /// there is no surviving <c>add</c>/<c>mul</c> instruction — instead the call carries the folded
    /// result. Asserting on that value is a stronger check than looking for the opcodes: it proves
    /// the no-precedence, left-to-right evaluation <c>((1 + 2) * 3) = 9</c> (not the math-precedence
    /// <c>1 + (2 * 3) = 7</c>).
    /// </summary>
    [Fact]
    public void Generate_ExampleProgram_EmitsExpectedIr()
    {
        using CodegenResult result = Generate(ExampleProgram);
        string ir = result.ToIrString();

        Assert.Contains("declare void @exit(i32)", ir);
        Assert.Contains("define i32 @main(", ir);
        // 1 + 2 * 3 evaluated strictly left-to-right = ((1 + 2) * 3) = 9.
        Assert.Contains("call void @exit(i32 9)", ir);
        // main is terminated with `ret i32 0`, not `unreachable`.
        Assert.Contains("ret i32 0", ir);
        Assert.DoesNotContain("unreachable", ir);
    }

    /// <summary>
    /// Explicit grouping is honoured: <c>1 + (2 * 3)</c> multiplies first, folding to <c>7</c> — the
    /// counterpart to the example's left-to-right <c>9</c>, so the two together prove parentheses
    /// are the only regrouping.
    /// </summary>
    [Fact]
    public void Generate_ParenthesizedGrouping_FoldsToSeven()
    {
        using CodegenResult result = Generate("extern fn exit(i32)\nfn main() { exit(1 + (2 * 3)) }\n");
        string ir = result.ToIrString();

        Assert.Contains("call void @exit(i32 7)", ir);
    }

    /// <summary>
    /// Subtraction and division evaluate left-to-right like the rest: <c>8 - 2 / 2</c> is
    /// <c>((8 - 2) / 2) = 3</c> (signed division), folded onto the call.
    /// </summary>
    [Fact]
    public void Generate_SubtractAndDivide_FoldLeftToRight()
    {
        using CodegenResult result = Generate("extern fn exit(i32)\nfn main() { exit(8 - 2 / 2) }\n");
        string ir = result.ToIrString();

        Assert.Contains("call void @exit(i32 3)", ir);
    }

    /// <summary>A bare-literal argument needs no arithmetic — just the constant and the call.</summary>
    [Fact]
    public void Generate_BareLiteralArgument_EmitsConstantCall()
    {
        using CodegenResult result = Generate("extern fn exit(i32)\nfn main() { exit(9) }\n");
        string ir = result.ToIrString();

        Assert.Contains("call void @exit(i32 9)", ir);
    }

    /// <summary>An extern with a non-void return type declares that return type.</summary>
    [Fact]
    public void Generate_ExternWithReturnType_DeclaresReturnType()
    {
        using CodegenResult result = Generate("extern fn read(i32) i32\nextern fn exit(i32)\nfn main() { exit(0) }\n");
        string ir = result.ToIrString();

        Assert.Contains("declare i32 @read(i32)", ir);
    }

    /// <summary>Every declared extern is emitted, even one main never calls.</summary>
    [Fact]
    public void Generate_UnusedExtern_StillDeclared()
    {
        using CodegenResult result = Generate("extern fn flush()\nextern fn exit(i32)\nfn main() { exit(0) }\n");
        string ir = result.ToIrString();

        Assert.Contains("declare void @flush()", ir);
        Assert.Contains("declare void @exit(i32)", ir);
    }

    /// <summary>Null program is a caller error, guarded like the rest of the public API.</summary>
    [Fact]
    public void Generate_NullProgram_Throws()
    {
        Assert.Throws<ArgumentNullException>(() => CodeGenerator.Generate(null!));
    }
}
