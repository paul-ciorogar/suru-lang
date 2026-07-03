using Suru.Compiler.Lexing;

namespace Suru.Compiler.Tests;

/// <summary>
/// Tests for <see cref="Token"/> — specifically <see cref="Token.FixedText"/>, the
/// canonical spelling of the fixed-text token kinds.
/// </summary>
public class TokenTests
{
    /// <summary>Each fixed-spelling kind maps to its literal source spelling.</summary>
    [Theory]
    [InlineData(TokenKind.Plus, "+")]
    [InlineData(TokenKind.Minus, "-")]
    [InlineData(TokenKind.Star, "*")]
    [InlineData(TokenKind.Slash, "/")]
    [InlineData(TokenKind.LParen, "(")]
    [InlineData(TokenKind.RParen, ")")]
    [InlineData(TokenKind.LBrace, "{")]
    [InlineData(TokenKind.RBrace, "}")]
    [InlineData(TokenKind.Extern, "extern")]
    [InlineData(TokenKind.Fn, "fn")]
    [InlineData(TokenKind.EndOfFile, "")]
    public void FixedText_FixedKind_ReturnsCanonicalSpelling(TokenKind kind, string expected)
    {
        Assert.Equal(expected, Token.FixedText(kind));
    }

    /// <summary>
    /// The variable-text kinds have no fixed spelling — their lexeme comes from the
    /// source — so asking for it is a programming error, not an empty string.
    /// </summary>
    [Theory]
    [InlineData(TokenKind.IntLiteral)]
    [InlineData(TokenKind.Identifier)]
    public void FixedText_VariableKind_Throws(TokenKind kind)
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => Token.FixedText(kind));
    }
}
