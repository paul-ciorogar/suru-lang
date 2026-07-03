namespace Suru.Compiler.Lexing;

/// <summary>
/// The lexical category of a <see cref="Token"/>.
/// </summary>
public enum TokenKind
{
    IntLiteral,
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Identifier,
    Extern,
    Fn,
    EndOfFile,
}

/// <summary>
/// One lexical token: a <see cref="TokenKind"/>, where it came from
/// (<see cref="Span"/>), and the source <see cref="Text"/> it stands for.
///
/// Tokens come in exactly two shapes, and this abstract base is what a token stream
/// holds so the two can be mixed in scan order:
/// </summary>
/// <param name="Kind">The lexical category of the token.</param>
/// <param name="Span">Where in the source the token is.</param>
public abstract record Token(TokenKind Kind, SourceSpan Span)
{
    /// <summary>
    /// The source lexeme this token stands for: the literal symbol (e.g. <c>"+"</c>)
    /// or keyword spelling for a <see cref="FixedToken"/>, the digits or name for a
    /// <see cref="TextToken"/>, and the empty string for end-of-file. Derived from the
    /// <see cref="Kind"/> for fixed tokens and stored for text tokens.
    /// </summary>
    public abstract string Text { get; }

    /// <summary>
    /// The canonical text of a token kind whose spelling is fixed by the grammar —
    /// the operators, brackets, keywords, and end-of-file. Their lexeme is fully
    /// determined by the <see cref="TokenKind"/>, so it is a compile-time constant
    /// (already pooled by the runtime) rather than something cut out of the source;
    /// </summary>
    /// <param name="kind">A fixed-text token kind.</param>
    /// <returns>The canonical spelling for <paramref name="kind"/>.</returns>
    /// <exception cref="ArgumentOutOfRangeException">
    /// Thrown for the variable-text kinds (<see cref="TokenKind.IntLiteral"/>,
    /// <see cref="TokenKind.Identifier"/>), whose lexeme must come from the source.
    /// </exception>
    public static string FixedText(TokenKind kind) => kind switch
    {
        TokenKind.Plus => "+",
        TokenKind.Minus => "-",
        TokenKind.Star => "*",
        TokenKind.Slash => "/",
        TokenKind.LParen => "(",
        TokenKind.RParen => ")",
        TokenKind.LBrace => "{",
        TokenKind.RBrace => "}",
        TokenKind.Extern => "extern",
        TokenKind.Fn => "fn",
        TokenKind.EndOfFile => "",
        _ => throw new ArgumentOutOfRangeException(
            nameof(kind), kind, "Kind has variable text; its lexeme comes from the source."),
    };
}

/// <summary>
/// A token whose spelling is fixed by its <see cref="TokenKind"/> — an operator,
/// bracket, keyword, or end-of-file.
/// </summary>
/// <param name="Kind">A fixed-text token kind (operator, bracket, keyword, or end-of-file).</param>
/// <param name="Span">Where in the source the token is.</param>
public sealed record FixedToken(TokenKind Kind, SourceSpan Span) : Token(Kind, Span)
{
    /// <inheritdoc />
    public override string Text => FixedText(Kind);
}

/// <summary>
/// A token whose lexeme is user-written and read from the source — an
/// <see cref="TokenKind.Identifier"/> or an <see cref="TokenKind.IntLiteral"/>.
/// </summary>
public sealed record TextToken : Token
{
    /// <inheritdoc />
    public override string Text { get; }

    /// <summary>Create a text token for a user-written lexeme.</summary>
    /// <param name="kind">A variable-text kind (<see cref="TokenKind.Identifier"/> or <see cref="TokenKind.IntLiteral"/>).</param>
    /// <param name="text">The token's source lexeme (ideally the interned instance).</param>
    /// <param name="span">Where in the source the token is.</param>
    public TextToken(TokenKind kind, string text, SourceSpan span) : base(kind, span)
    {
        Text = text;
    }
}
