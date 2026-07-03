using Suru.Compiler.Lexing;

namespace Suru.Compiler.Tests;

/// <summary>
/// Tests for <see cref="Tokens"/> — the append-only, text-interning token buffer.
///
/// These pin the interning contract that motivates the class: tokens with the same
/// text share a single canonical <see cref="string"/> instance (so distinct-string
/// allocations are bounded by the number of distinct lexemes, not tokens), while
/// different texts stay distinct. The collection also behaves as an ordinary
/// <see cref="IReadOnlyList{Token}"/>.
/// </summary>
public class TokensTests
{
    private static readonly SourceSpan AnySpan = new(1, 1, 0, 0);

    /// <summary>
    /// Two tokens added with equal text share the exact same string reference — the
    /// point of the registry. (A fresh <c>new string</c> is used for the second add so
    /// the result cannot be explained by the compiler pooling identical literals.)
    /// </summary>
    [Fact]
    public void AddToken_SameText_SharesOneStringInstance()
    {
        Tokens tokens = new();
        string first = "counter";
        string second = new(first.ToCharArray()); // distinct instance, equal value

        tokens.AddToken(TokenKind.Identifier, first, AnySpan);
        tokens.AddToken(TokenKind.Identifier, second, AnySpan);

        Assert.Equal(2, tokens.Count);
        Assert.Same(tokens[0].Text, tokens[1].Text);
        Assert.Equal("counter", tokens[0].Text);
        Assert.Equal(1, tokens.DistinctTextCount);
    }

    /// <summary>Different texts are kept distinct and each counts once in the registry.</summary>
    [Fact]
    public void AddToken_DifferentText_KeepsDistinctEntries()
    {
        Tokens tokens = new();

        tokens.AddToken(TokenKind.Identifier, "a", AnySpan);
        tokens.AddToken(TokenKind.Identifier, "b", AnySpan);
        tokens.AddToken(TokenKind.Identifier, "a", AnySpan);

        Assert.Equal(3, tokens.Count);
        Assert.Same(tokens[0].Text, tokens[2].Text);
        Assert.NotSame(tokens[0].Text, tokens[1].Text);
        // Only "a" and "b" are distinct lexemes.
        Assert.Equal(2, tokens.DistinctTextCount);
    }

    /// <summary>The kind and span passed to <see cref="Tokens.AddToken"/> are preserved verbatim.</summary>
    [Fact]
    public void AddToken_PreservesKindAndSpan()
    {
        Tokens tokens = new();
        SourceSpan span = new(2, 5, 17, 3);

        tokens.AddToken(TokenKind.Identifier, "main", span);

        Assert.Equal(TokenKind.Identifier, tokens[0].Kind);
        Assert.Equal(span, tokens[0].Span);
        Assert.Equal("main", tokens[0].Text);
    }

    /// <summary>
    /// Fixed-spelling tokens are stored verbatim and NOT interned: adding several
    /// leaves the registry empty, so they never inflate the distinct-lexeme pool.
    /// </summary>
    [Fact]
    public void AddFixedToken_DoesNotIntern()
    {
        Tokens tokens = new();

        tokens.AddFixedToken(TokenKind.Plus, AnySpan);
        tokens.AddFixedToken(TokenKind.Fn, AnySpan);
        tokens.AddFixedToken(TokenKind.LParen, AnySpan);

        Assert.Equal(3, tokens.Count);
        Assert.Equal("+", tokens[0].Text);
        Assert.Equal("fn", tokens[1].Text);
        Assert.Equal("(", tokens[2].Text);
        // The interning registry was never touched.
        Assert.Equal(0, tokens.DistinctTextCount);
    }

    /// <summary>
    /// The two add paths produce the two concrete token shapes: an interned
    /// <see cref="TextToken"/> for user-written lexemes and a text-less
    /// <see cref="FixedToken"/> for fixed spellings. A fixed token still surfaces its
    /// lexeme through <see cref="Token.Text"/> (derived from its kind), so consumers can
    /// read text uniformly while pattern matching on the shape when it matters.
    /// </summary>
    [Fact]
    public void Add_ProducesTheMatchingTokenShape()
    {
        Tokens tokens = new();

        tokens.AddToken(TokenKind.Identifier, "main", AnySpan);
        tokens.AddFixedToken(TokenKind.Plus, AnySpan);

        Assert.IsType<TextToken>(tokens[0]);
        Assert.Equal("main", tokens[0].Text);

        FixedToken plus = Assert.IsType<FixedToken>(tokens[1]);
        // The fixed token stores no lexeme of its own; it derives it from the kind.
        Assert.Equal("+", plus.Text);
    }

    /// <summary>The buffer enumerates in the order tokens were added, mixing both add paths.</summary>
    [Fact]
    public void Tokens_EnumeratesInInsertionOrder()
    {
        Tokens tokens = new();
        tokens.AddToken(TokenKind.IntLiteral, "1", AnySpan);
        tokens.AddFixedToken(TokenKind.Plus, AnySpan);
        tokens.AddToken(TokenKind.IntLiteral, "2", AnySpan);

        TokenKind[] kinds = tokens.Select(t => t.Kind).ToArray();

        Assert.Equal([TokenKind.IntLiteral, TokenKind.Plus, TokenKind.IntLiteral], kinds);
    }
}
