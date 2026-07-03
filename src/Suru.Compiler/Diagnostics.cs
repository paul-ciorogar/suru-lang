namespace Suru.Compiler;

/// <summary>
/// A located region of Suru source text.
///
/// Every token and every diagnostic carries a <see cref="SourceSpan"/> so that
/// errors can point at exactly where in the source they occurred. The line and
/// column are 1-based (what an editor shows), while the offset is a 0-based index
/// into the source string (what a scanner counts). 
/// </summary>
/// <param name="Line">1-based line number of the span's first character.</param>
/// <param name="Column">1-based column number of the span's first character.</param>
/// <param name="Offset">0-based index of the span's first character in the source string.</param>
/// <param name="Length">Length of the span in characters (0 for an empty span, e.g. end-of-file).</param>
public readonly record struct SourceSpan(int Line, int Column, int Offset, int Length);

/// <summary>
/// A single located problem found while compiling — the reusable unit of error
/// reporting shared across the pipeline.
/// </summary>
/// <param name="Span">Where in the source the problem is.</param>
/// <param name="Message">Human-readable description of the problem.</param>
public readonly record struct Diagnostic(SourceSpan Span, string Message)
{
    /// <summary>
    /// Construct a diagnostic. Named <c>Error</c> to read clearly at the call site.
    /// </summary>
    public static Diagnostic Error(SourceSpan span, string message) => new(span, message);
}
