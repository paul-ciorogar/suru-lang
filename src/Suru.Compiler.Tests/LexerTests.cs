using Suru.Compiler.Lexing;

namespace Suru.Compiler.Tests;

/// <summary>
/// Tests for the <see cref="Lexer"/> — the first pass of the compile pipeline.
/// </summary>
public class LexerTests
{
    private const string ExampleProgram =
        "extern exit\n" +
        "\n" +
        "fn main() {\n" +
        "    exit(1 + 2 * 3)\n" +
        "}\n";

    /// <summary>
    /// The headline "Done" criterion: the example program lexes into the exact
    /// expected token-kind stream (terminated by end-of-file) with no diagnostics.
    /// </summary>
    [Fact]
    public void Tokenize_ExampleProgram_ProducesExpectedStreamWithNoDiagnostics()
    {
        LexResult result = Lexer.Tokenize(ExampleProgram);

        Assert.Empty(result.Diagnostics);

        TokenKind[] expected =
        [
            TokenKind.Extern,      // extern
            TokenKind.Identifier,  // exit
            TokenKind.Fn,          // fn
            TokenKind.Identifier,  // main
            TokenKind.LParen,      // (
            TokenKind.RParen,      // )
            TokenKind.LBrace,      // {
            TokenKind.Identifier,  // exit
            TokenKind.LParen,      // (
            TokenKind.IntLiteral,  // 1
            TokenKind.Plus,        // +
            TokenKind.IntLiteral,  // 2
            TokenKind.Star,        // *
            TokenKind.IntLiteral,  // 3
            TokenKind.RParen,      // )
            TokenKind.RBrace,      // }
            TokenKind.EndOfFile,
        ];

        Assert.Equal(expected, result.Tokens.Select(t => t.Kind));
    }

    /// <summary>
    /// The keyword table maps <c>extern</c>/<c>fn</c> to keyword kinds while
    /// <c>main</c>/<c>exit</c> (and other names) stay ordinary identifiers — the
    /// entry point is just the function named <c>main</c>, not a keyword.
    /// </summary>
    [Theory]
    [InlineData("extern", TokenKind.Extern)]
    [InlineData("fn", TokenKind.Fn)]
    [InlineData("main", TokenKind.Identifier)]
    [InlineData("exit", TokenKind.Identifier)]
    [InlineData("printf", TokenKind.Identifier)]
    [InlineData("_x1", TokenKind.Identifier)]
    public void Tokenize_Word_ClassifiesKeywordVersusIdentifier(string word, TokenKind expected)
    {
        LexResult result = Lexer.Tokenize(word);

        Assert.Empty(result.Diagnostics);
        // The word, then end-of-file.
        Assert.Equal(2, result.Tokens.Count);
        Assert.Equal(expected, result.Tokens[0].Kind);
        Assert.Equal(word, result.Tokens[0].Text);
    }

    /// <summary>A multi-digit integer literal is one token whose text is the digits.</summary>
    [Fact]
    public void Tokenize_MultiDigitNumber_CapturesLexeme()
    {
        LexResult result = Lexer.Tokenize("255");

        Assert.Empty(result.Diagnostics);
        Assert.Equal(TokenKind.IntLiteral, result.Tokens[0].Kind);
        Assert.Equal("255", result.Tokens[0].Text);
    }

    /// <summary>Each single-character operator and bracket maps to its own kind.</summary>
    [Theory]
    [InlineData("+", TokenKind.Plus)]
    [InlineData("-", TokenKind.Minus)]
    [InlineData("*", TokenKind.Star)]
    [InlineData("/", TokenKind.Slash)]
    [InlineData("(", TokenKind.LParen)]
    [InlineData(")", TokenKind.RParen)]
    [InlineData("{", TokenKind.LBrace)]
    [InlineData("}", TokenKind.RBrace)]
    public void Tokenize_Symbol_MapsToKind(string symbol, TokenKind expected)
    {
        LexResult result = Lexer.Tokenize(symbol);

        Assert.Empty(result.Diagnostics);
        Assert.Equal(expected, result.Tokens[0].Kind);
        Assert.Equal(symbol, result.Tokens[0].Text);
    }

    /// <summary>
    /// Position tracking survives newlines (including a blank line): the <c>fn</c>
    /// token in the example sits at line 3, column 1, offset 13.
    /// </summary>
    [Fact]
    public void Tokenize_TokenAfterNewlines_ReportsCorrectPosition()
    {
        LexResult result = Lexer.Tokenize(ExampleProgram);

        Token fn = result.Tokens.Single(t => t.Kind == TokenKind.Fn);

        Assert.Equal(3, fn.Span.Line);
        Assert.Equal(1, fn.Span.Column);
        // "extern exit\n" (12) + "\n" (1) = 13 characters precede `fn`.
        Assert.Equal(13, fn.Span.Offset);
        Assert.Equal(2, fn.Span.Length);
    }

    /// <summary>
    /// The column advances within a line: the <c>*</c> on the body line reports a
    /// column past the start, with a length-1 span.
    /// </summary>
    [Fact]
    public void Tokenize_TokenMidLine_ReportsColumnAndOffset()
    {
        // A single line so offsets and columns coincide (both 1-based-ish).
        LexResult result = Lexer.Tokenize("1 + 20 * 3");

        Token star = result.Tokens.Single(t => t.Kind == TokenKind.Star);

        Assert.Equal(1, star.Span.Line);
        Assert.Equal(8, star.Span.Column);   // index 7, column 8 (1-based)
        Assert.Equal(7, star.Span.Offset);
        Assert.Equal(1, star.Span.Length);
    }

    /// <summary>
    /// An unexpected character is reported as a located diagnostic (not thrown),
    /// and the surrounding valid tokens still lex — the continue-on-error contract.
    /// </summary>
    [Fact]
    public void Tokenize_UnexpectedCharacter_ReportsDiagnosticAndKeepsLexing()
    {
        LexResult result = Lexer.Tokenize("1 @ 2");

        // Exactly one diagnostic, pointing at the '@'.
        Diagnostic diag = Assert.Single(result.Diagnostics);
        Assert.Equal(2, diag.Span.Offset);
        Assert.Equal(1, diag.Span.Line);
        Assert.Equal(3, diag.Span.Column);
        Assert.Equal(1, diag.Span.Length);
        Assert.Contains("@", diag.Message);

        // The '@' produced no token, but the two integers around it still lexed.
        TokenKind[] expected = [TokenKind.IntLiteral, TokenKind.IntLiteral, TokenKind.EndOfFile];
        Assert.Equal(expected, result.Tokens.Select(t => t.Kind));
    }

    /// <summary>Empty and whitespace-only input yields just an end-of-file token, no diagnostics.</summary>
    [Theory]
    [InlineData("")]
    [InlineData("   ")]
    [InlineData(" \t\r\n ")]
    public void Tokenize_BlankInput_ProducesOnlyEndOfFile(string source)
    {
        LexResult result = Lexer.Tokenize(source);

        Assert.Empty(result.Diagnostics);
        Token only = Assert.Single(result.Tokens);
        Assert.Equal(TokenKind.EndOfFile, only.Kind);
        Assert.Equal(string.Empty, only.Text);
    }

    /// <summary>Null source is a caller error, guarded like the rest of the public API.</summary>
    [Fact]
    public void Tokenize_NullSource_Throws()
    {
        Assert.Throws<ArgumentNullException>(() => Lexer.Tokenize(null!));
    }

    /// <summary>
    /// Interning is wired through the lexer: the two <c>exit</c> identifiers in the
    /// example program (the <c>extern</c> name and the call target) share one string
    /// instance rather than each holding a separate substring of the source.
    /// </summary>
    [Fact]
    public void Tokenize_RepeatedIdentifier_SharesOneStringInstance()
    {
        LexResult result = Lexer.Tokenize(ExampleProgram);

        Token[] exits = result.Tokens
            .Where(t => t.Kind == TokenKind.Identifier && t.Text == "exit")
            .ToArray();

        Assert.Equal(2, exits.Length);
        Assert.Same(exits[0].Text, exits[1].Text);
    }

    /// <summary>
    /// Only variable-text tokens are interned: the example program's registry holds
    /// exactly its distinct identifiers (<c>exit</c>, <c>main</c>) and int literals
    /// (<c>1</c>, <c>2</c>, <c>3</c>) — 5 in all. The keywords (<c>extern</c>/<c>fn</c>),
    /// operators, and brackets have fixed spellings and never enter the pool.
    /// </summary>
    [Fact]
    public void Tokenize_ExampleProgram_InternsOnlyVariableTextTokens()
    {
        LexResult result = Lexer.Tokenize(ExampleProgram);

        Assert.Equal(5, result.Tokens.DistinctTextCount);
    }
}
