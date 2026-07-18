namespace Suru.Compiler;

/// <summary>
/// The public entry point into the Suru compiler.
public static class Compiler
{
    /// <summary>
    /// Compile a single Suru source file.
    ///
    /// </summary>
    /// <param name="sourcePath">Path to the <c>.suru</c> source file to compile.</param>
    /// <returns>
    /// A <see cref="CompileResult"/> describing the outcome.
    /// </returns>
    /// <exception cref="ArgumentException">
    /// Thrown when <paramref name="sourcePath"/> is null, empty, or whitespace.
    /// </exception>
    public static CompileResult Compile(string sourcePath)
    {
        // Guard the contract now so callers (the CLI) get a clear error even
        // while the real pipeline is still a stub.
        if (string.IsNullOrWhiteSpace(sourcePath))
        {
            throw new ArgumentException("Source path must be a non-empty path.", nameof(sourcePath));
        }

        // No compilation pipeline exists yet. Report that plainly rather than
        // pretending success.
        return CompileResult.NotImplemented(
            $"Suru compilation is not implemented yet (requested: '{sourcePath}').");
    }
}


public readonly record struct CompileResult(
    bool Succeeded,
    string Message,
    string? OutputPath = null,
    IReadOnlyList<Diagnostic>? Diagnostics = null)
{

    public IReadOnlyList<Diagnostic> Diagnostics { get; init; } = Diagnostics ?? [];


    public static CompileResult NotImplemented(string message) => new(false, message);


    public static CompileResult Success(
        string outputPath, string message, IReadOnlyList<Diagnostic>? diagnostics = null) =>
        new(true, message, outputPath, diagnostics);


    public static CompileResult Failure(string message, IReadOnlyList<Diagnostic>? diagnostics = null) =>
        new(false, message, null, diagnostics);
}
