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

/// <summary>
/// The outcome of a <see cref="Compiler.Compile"/> call.
///
/// A small immutable record so callers can branch on success without exceptions
/// for the ordinary "it didn't compile" case. The set of states is intentionally
/// minimal — success plus a message — and will grow (diagnostics, output
/// artifacts) as the pipeline is built out.
/// </summary>
/// <param name="Succeeded">True if compilation produced output.</param>
/// <param name="Message">A human-readable description of the outcome.</param>
public readonly record struct CompileResult(bool Succeeded, string Message)
{
    /// <summary>
    /// Construct the "pipeline not built yet" result: not a success, but not an
    /// error the caller did anything wrong to cause.
    /// </summary>
    public static CompileResult NotImplemented(string message) => new(false, message);
}
