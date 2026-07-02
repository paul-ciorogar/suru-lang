namespace Suru.Compiler.Tests;

/// <summary>
/// Tests for the public compiler entry point (<see cref="Compiler"/>).
///
/// There is no real compilation pipeline yet, so these tests pin the
/// placeholder's observable contract: the input guard, and the "not implemented"
/// outcome. They exist mainly to prove the test project builds, references the
/// compiler library, and runs green under `scripts/test` — the assertions are
/// tightened as the real lex → parse → analyse → codegen flow lands.
/// </summary>
public class CompilerTests
{
    /// <summary>
    /// Compiling any path returns a non-success
    /// <see cref="CompileResult"/> whose message names the requested source —
    /// the documented placeholder behaviour.
    /// </summary>
    [Fact]
    public void Compile_WhileUnimplemented_ReportsNotImplemented()
    {
        CompileResult result = Compiler.Compile("example.suru");

        Assert.False(result.Succeeded);
        Assert.Contains("example.suru", result.Message);
    }

    /// <summary>
    /// The entry point guards its contract even while the pipeline is a stub:
    /// a null/empty/whitespace path is a caller error, not a compile failure.
    /// </summary>
    [Theory]
    [InlineData("")]
    [InlineData("   ")]
    [InlineData(null)]
    public void Compile_WithBlankPath_Throws(string? sourcePath)
    {
        Assert.Throws<ArgumentException>(() => Compiler.Compile(sourcePath!));
    }
}
