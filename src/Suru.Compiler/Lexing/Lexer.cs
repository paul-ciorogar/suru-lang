namespace Suru.Compiler.Lexing;

/// <summary>
/// The outcome of a <see cref="Lexer.Tokenize"/> call: the token stream plus any
/// problems found while producing it.
///
/// <see cref="Tokens"/> always ends with a single <see cref="TokenKind.EndOfFile"/>
/// token, even for empty or all-whitespace input, so the parser has a guaranteed
/// terminator. <see cref="Diagnostics"/> is empty when the source lexed cleanly;
/// because the lexer records problems rather than throwing, a caller gets a usable
/// token stream AND the list of what went wrong from the same run.
/// </summary>
/// <param name="Tokens">The lexed tokens, always terminated by an end-of-file token.</param>
/// <param name="Diagnostics">Problems found while lexing; empty on clean input.</param>
public readonly record struct LexResult(
    Tokens Tokens,
    IReadOnlyList<Diagnostic> Diagnostics);

/// <summary>
/// The lexer: turns Suru source text into a flat stream of <see cref="Token"/>s —
/// the first pass of the compile pipeline.
///
/// The scan is a single left-to-right pass that skips whitespace and cuts one token at
/// a time. Bad characters are reported as structured <see cref="Diagnostic"/>s and
/// skipped, so one malformed character does not abort the whole scan — the lexer
/// resynchronises at the next character and keeps going.
/// </summary>
public static class Lexer
{
    /// <summary>
    /// Tokenize an entire Suru source string.
    /// </summary>
    /// <param name="source">The full source text. May be empty.</param>
    /// <returns>
    /// A <see cref="LexResult"/> whose <see cref="LexResult.Tokens"/> is terminated
    /// by an <see cref="TokenKind.EndOfFile"/> token, and whose
    /// <see cref="LexResult.Diagnostics"/> lists any unexpected characters.
    /// </returns>
    /// <exception cref="ArgumentNullException">
    /// Thrown when <paramref name="source"/> is null.
    /// </exception>
    public static LexResult Tokenize(string source)
    {
        ArgumentNullException.ThrowIfNull(source);

        // A single Scanner instance owns the mutable cursor state for this call.
        // Keeping it in an instance (rather than static fields) makes Tokenize
        // trivially re-entrant / thread-safe.
        Scanner scanner = new(source);
        return scanner.Run();
    }

    /// <summary>
    /// The mutable cursor over the source for one <see cref="Tokenize"/> call.
    ///
    /// Every character is consumed through <see cref="Advance"/>, which is the one
    /// place line/column bookkeeping lives — so position tracking cannot drift.
    /// </summary>
    private sealed class Scanner
    {
        private readonly string _source;
        private readonly Tokens _tokens = new();
        private readonly List<Diagnostic> _diagnostics = new();

        // Cursor: _offset indexes the next character to read; _line/_column are
        // its 1-based editor position.
        private int _offset;
        private int _line = 1;
        private int _column = 1;

        public Scanner(string source) => _source = source;

        /// <summary>Run the scan to completion and return the collected result.</summary>
        public LexResult Run()
        {
            while (!AtEnd)
            {
                char c = Peek();

                // Whitespace separates tokens but produces none of its own.
                if (IsWhitespace(c))
                {
                    Advance();
                    continue;
                }

                // Numbers, then names/keywords, then the fixed single-char tokens.
                if (IsDigit(c))
                {
                    ScanNumber();
                }
                else if (IsIdentifierStart(c))
                {
                    ScanIdentifierOrKeyword();
                }
                else
                {
                    ScanSymbolOrError(c);
                }
            }

            // The stream always ends with a zero-length end-of-file token at the
            // current (final) position. Its text is fixed (empty), so it is not interned.
            _tokens.AddFixedToken(
                TokenKind.EndOfFile, new SourceSpan(_line, _column, _offset, 0));

            return new LexResult(_tokens, _diagnostics);
        }

        /// <summary>Consume a run of ASCII digits into an <see cref="TokenKind.IntLiteral"/>.</summary>
        private void ScanNumber()
        {
            int startOffset = _offset;
            int startLine = _line;
            int startColumn = _column;

            while (!AtEnd && IsDigit(Peek()))
            {
                Advance();
            }

            Emit(TokenKind.IntLiteral, startOffset, startLine, startColumn);
        }

        /// <summary>
        /// Consume an identifier run and classify it. 
        /// </summary>
        private void ScanIdentifierOrKeyword()
        {
            int startOffset = _offset;
            int startLine = _line;
            int startColumn = _column;

            while (!AtEnd && IsIdentifierPart(Peek()))
            {
                Advance();
            }

            // Classify by matching the source slice directly — a span switch avoids
            // allocating a substring just to compare against the keyword table.
            ReadOnlySpan<char> text = _source.AsSpan(startOffset, _offset - startOffset);
            TokenKind kind = text switch
            {
                "extern" => TokenKind.Extern,
                "fn" => TokenKind.Fn,
                "i8" => TokenKind.I8,
                "i16" => TokenKind.I16,
                "i32" => TokenKind.I32,
                "i64" => TokenKind.I64,
                "u8" => TokenKind.U8,
                "u16" => TokenKind.U16,
                "u32" => TokenKind.U32,
                "u64" => TokenKind.U64,
                "void" => TokenKind.Void,
                _ => TokenKind.Identifier,
            };

            // Only identifiers are text-carrying (interned); every keyword — including the
            // built-in type names — has a single fixed spelling, so it takes the non-interning path.
            if (kind is TokenKind.Identifier)
            {
                Emit(kind, startOffset, startLine, startColumn);
            }
            else
            {
                EmitFixed(kind, startOffset, startLine, startColumn);
            }
        }

        /// <summary>
        /// Handle a single character that is either a fixed one-char token
        /// (operator/bracket) or, if unrecognised, an unexpected-character
        /// diagnostic. Either way exactly one character is consumed so the scan
        /// makes progress and resynchronises.
        /// </summary>
        private void ScanSymbolOrError(char c)
        {
            int startOffset = _offset;
            int startLine = _line;
            int startColumn = _column;

            TokenKind? kind = c switch
            {
                '+' => TokenKind.Plus,
                '-' => TokenKind.Minus,
                '*' => TokenKind.Star,
                '/' => TokenKind.Slash,
                '(' => TokenKind.LParen,
                ')' => TokenKind.RParen,
                '{' => TokenKind.LBrace,
                '}' => TokenKind.RBrace,
                ',' => TokenKind.Comma,
                _ => null,
            };

            Advance();

            if (kind is TokenKind resolved)
            {
                // Operators and brackets have fixed spellings — not interned.
                EmitFixed(resolved, startOffset, startLine, startColumn);
            }
            else
            {
                // Unknown character: record where and what, then carry on.
                _diagnostics.Add(Diagnostic.Error(
                    new SourceSpan(startLine, startColumn, startOffset, 1),
                    $"unexpected character '{c}'"));
            }
        }

        /// <summary>
        /// Append an <em>interned</em> token spanning from <paramref name="startOffset"/>
        /// (at the given line/column) up to the current cursor. For the variable-text
        /// kinds — identifiers and int literals — whose lexeme is user-written source.
        /// The slice is handed to <see cref="Tokens.AddToken"/> so its text is interned
        /// rather than freshly allocated per occurrence.
        /// </summary>
        private void Emit(TokenKind kind, int startOffset, int startLine, int startColumn)
        {
            int length = _offset - startOffset;
            _tokens.AddToken(
                kind,
                _source.AsSpan(startOffset, length),
                new SourceSpan(startLine, startColumn, startOffset, length));
        }

        /// <summary>
        /// Append a fixed-spelling token (operator, bracket, or keyword) spanning from
        /// <paramref name="startOffset"/> up to the current cursor. Its lexeme is the
        /// kind's compile-time constant, so it goes to <see cref="Tokens.AddFixedToken"/>
        /// as a <see cref="FixedToken"/> — which stores no text and never touches the
        /// interning registry.
        /// </summary>
        private void EmitFixed(TokenKind kind, int startOffset, int startLine, int startColumn)
        {
            int length = _offset - startOffset;
            _tokens.AddFixedToken(
                kind,
                new SourceSpan(startLine, startColumn, startOffset, length));
        }

        /// <summary>True once every character has been consumed.</summary>
        private bool AtEnd => _offset >= _source.Length;

        /// <summary>Look at the current character without consuming it. Caller must ensure not <see cref="AtEnd"/>.</summary>
        private char Peek() => _source[_offset];

        /// <summary>
        /// Consume the current character, advancing the cursor and keeping the
        /// line/column position correct. A newline moves to the next line and
        /// resets the column; every other character advances the column by one.
        /// This is the single source of truth for position tracking.
        /// </summary>
        private void Advance()
        {
            char c = _source[_offset];
            _offset++;
            if (c == '\n')
            {
                _line++;
                _column = 1;
            }
            else
            {
                _column++;
            }
        }

        // --- Character classes (ASCII-only; the language is currently ASCII). ---

        private static bool IsWhitespace(char c) => c is ' ' or '\t' or '\r' or '\n';

        private static bool IsDigit(char c) => c is >= '0' and <= '9';

        private static bool IsIdentifierStart(char c) =>
            c is (>= 'a' and <= 'z') or (>= 'A' and <= 'Z') or '_';

        private static bool IsIdentifierPart(char c) => IsIdentifierStart(c) || IsDigit(c);
    }
}
