using Suru.Compiler.Lexing;

namespace Suru.Compiler.Parsing;

/// <summary>
/// The outcome of a <see cref="Parser.Parse"/> call: the parsed program plus any
/// problems found while producing it.
///
/// <see cref="Program"/> is the parsed <see cref="SuruProgram"/> when a usable entry
/// point was recovered, or <c>null</c> when the source had no function definition to
/// build one from. <see cref="Diagnostics"/> is empty when the source parsed cleanly;
/// one run yields both a best-effort tree AND the full list of what went wrong.
/// </summary>
/// <param name="Program">The parsed program, or <c>null</c> if none could be recovered.</param>
/// <param name="Diagnostics">Problems found while parsing; empty on clean input.</param>
public readonly record struct ParseResult(
    SuruProgram? Program,
    IReadOnlyList<Diagnostic> Diagnostics);

/// <summary>
/// The parser: turns the lexer's <see cref="Token"/> stream into a
/// <see cref="SuruProgram"/> abstract syntax tree.
///
/// Recursive-descent parser.
///
/// Suru has <em>no operator precedence</em>: every binary math operator shares one level and
/// binds left-to-right, so <see cref="ParseExpr"/> is a flat left-fold — <c>1 + 2 * 3</c>
/// nests as <c>((1 + 2) * 3)</c>, not <c>1 + (2 * 3)</c>. Parentheses are the only way to
/// regroup.
/// </summary>
public static class Parser
{
    /// <summary>
    /// Parse a token stream into a <see cref="SuruProgram"/>.
    /// </summary>
    /// <param name="tokens">The tokens to parse</param>
    /// <returns>
    /// A <see cref="ParseResult"/> whose <see cref="ParseResult.Program"/> is the parsed
    /// tree (or <c>null</c> if none was recovered) and whose
    /// <see cref="ParseResult.Diagnostics"/> lists every problem found.
    /// </returns>
    /// <exception cref="ArgumentNullException">
    /// Thrown when <paramref name="tokens"/> is null.
    /// </exception>
    public static ParseResult Parse(Tokens tokens)
    {
        ArgumentNullException.ThrowIfNull(tokens);

        // A single Cursor owns the mutable position + diagnostics for this call,
        // keeping Parse trivially re-entrant.
        Cursor cursor = new(tokens);
        return cursor.Run();
    }

    /// <summary>
    /// The internal control-flow signal raised when the parser hits a token it cannot
    /// use. It carries no data — the located <see cref="Diagnostic"/> is already recorded
    /// by the time this is thrown — and exists only to unwind the recursive-descent call
    /// stack up to the nearest recovery boundary, which catches it and resynchronises.
    /// It never escapes <see cref="Parse"/>.
    /// </summary>
    private sealed class ParseError : Exception;

    /// <summary>
    /// The mutable cursor over the token stream for one <see cref="Parse"/> call: the
    /// current position plus the growing diagnostic list. Every token is read through
    /// <see cref="Peek"/>/<see cref="Advance"/> so position handling lives in one place.
    /// </summary>
    private sealed class Cursor(Tokens tokens)
    {
        private readonly Tokens _tokens = tokens;
        private readonly List<Diagnostic> _diagnostics = new();
        private int _position;

        /// <summary>Run the parse to completion and return the collected result.</summary>
        public ParseResult Run()
        {
            SuruProgram? program = ParseProgram();
            return new ParseResult(program, _diagnostics);
        }

        // --- Grammar productions ---

        /// <summary>
        /// Parse the whole program.
        ///
        /// The top-level loop is the coarse recovery boundary: each declaration is parsed
        /// inside a try/catch so a broken one is reported, then <see cref="Synchronize"/>
        /// skips to the next declaration and the loop continues — the rest of the file is
        /// still parsed.
        /// </summary>
        private SuruProgram? ParseProgram()
        {
            List<ExternDeclaration> externs = new();
            List<FunctionDefinition> functions = new();

            while (!AtEnd)
            {
                try
                {
                    if (Check(TokenKind.Extern))
                    {
                        externs.Add(ParseExtern());
                    }
                    else if (Check(TokenKind.Fn))
                    {
                        functions.Add(ParseFunction());
                    }
                    else
                    {
                        throw Error(
                            Peek(),
                            "expected an 'extern' declaration or a function definition");
                    }
                }
                catch (ParseError)
                {
                    Synchronize();
                }
            }

            return BuildProgram(externs, functions);
        }

        /// <summary>
        /// Resolve the parsed declarations into a program. A program may define any number
        /// of functions, but exactly one must be named <see cref="SuruProgram.EntryPointName"/>
        /// </summary>
        private SuruProgram? BuildProgram(
            List<ExternDeclaration> externs, List<FunctionDefinition> functions)
        {
            FunctionDefinition? main = null;
            foreach (FunctionDefinition fn in functions)
            {
                if (fn.Name != SuruProgram.EntryPointName)
                {
                    continue;
                }

                if (main is null)
                {
                    main = fn;
                }
                else
                {
                    _diagnostics.Add(Diagnostic.Error(
                        fn.Span,
                        $"a program may define only one '{SuruProgram.EntryPointName}' function"));
                }
            }

            if (main is null)
            {
                if (_diagnostics.Count == 0)
                {
                    _diagnostics.Add(Diagnostic.Error(
                        Peek().Span,
                        $"a program must define a '{SuruProgram.EntryPointName}' function"));
                }
                return null;
            }

            return new SuruProgram(externs, functions, main);
        }

        /// <summary>
        /// Parse extern declaration.
        /// </summary>
        private ExternDeclaration ParseExtern()
        {
            Token externKw = Expect(TokenKind.Extern, "'extern'");
            Expect(TokenKind.Fn, "'fn' after 'extern'");
            Token name = Expect(TokenKind.Identifier, "a function name after 'fn'");
            Expect(TokenKind.LParen, "'(' after the function name");

            List<TypeReference> parameters = new();
            if (!Check(TokenKind.RParen))
            {
                do
                {
                    parameters.Add(ParseType());
                }
                while (Match(TokenKind.Comma));
            }

            Token close = Expect(TokenKind.RParen, "')' to close the parameter list");

            // The return type is optional; when the source omits it the extern returns void.
            // The synthesised reference carries the close-paren span for any later diagnostic.
            TypeReference returnType = IsBuiltInTypeKind(Peek().Kind)
                ? ParseType()
                : new TypeReference("void", close.Span);

            return new ExternDeclaration(
                name.Text, parameters, returnType, SpanFrom(externKw.Span, returnType.Span));
        }

        /// <summary>
        /// Parse a type reference — one of the reserved type keywords. A non-type token here
        /// (e.g. a stray identifier) is where an unknown/misspelled type is caught.
        /// </summary>
        private TypeReference ParseType()
        {
            if (!IsBuiltInTypeKind(Peek().Kind))
            {
                throw Error(Peek(), "expected a type name");
            }

            Token token = Advance();
            return new TypeReference(token.Text, token.Span);
        }

        /// <summary>
        /// Parse <c>fn IDENT '(' ')' '{' Statement* '}'</c>.
        /// </summary>
        private FunctionDefinition ParseFunction()
        {
            Token functionKeyword = Expect(TokenKind.Fn, "'fn'");
            Token name = Expect(TokenKind.Identifier, "a function name");
            Expect(TokenKind.LParen, "'(' after the function name");
            Expect(TokenKind.RParen, "')'");
            Block body = ParseBlock();
            return new FunctionDefinition(name.Text, body, SpanFrom(functionKeyword.Span, body.Span));
        }

        /// <summary>
        /// Parse <c>'{' Statement* '}'</c>. The statement loop is a recovery boundary:
        /// a failed statement synchronizes to the next one so one pass reports every
        /// independent mistake in the block.
        /// </summary>
        private Block ParseBlock()
        {
            Token open = Expect(TokenKind.LBrace, "'{' to open the block");

            List<Statement> statements = new();
            while (!Check(TokenKind.RBrace) && !AtEnd)
            {
                try
                {
                    statements.Add(ParseStatement());
                }
                catch (ParseError)
                {
                    SynchronizeStatement();
                }
            }

            Token close = Expect(TokenKind.RBrace, "'}' to close the block");
            return new Block(statements, SpanFrom(open, close));
        }

        /// <summary>
        /// Parse one statement.
        /// </summary>
        private Statement ParseStatement()
        {
            CallExpression call = ParseCall();
            return new ExpressionStatement(call, call.Span);
        }

        private CallExpression ParseCall()
        {
            Token name = Expect(TokenKind.Identifier, "a function name to call");
            IdentifierExpression callee = new(name.Text, name.Span);
            Expect(TokenKind.LParen, "'(' after the callee");

            List<Expression> arguments = new();
            if (!Check(TokenKind.RParen))
            {
                do
                {
                    arguments.Add(ParseExpr());
                }
                while (Match(TokenKind.Comma));
            }

            Token close = Expect(TokenKind.RParen, "')' to close the call");
            return new CallExpression(callee, arguments, SpanFrom(name, close));
        }

        /// <summary>
        /// Parse an arithmetic expression as a flat, left-associative fold over the binary
        /// operators — Suru has no operator precedence, so a single loop (not a precedence
        /// climb) is exactly right. Each operator wraps the accumulated left-hand tree,
        /// giving the left-to-right nesting the language requires.
        /// </summary>
        private Expression ParseExpr()
        {
            Expression left = ParsePrimary();

            while (TryBinaryOperator(Peek().Kind, out BinaryOperator op))
            {
                Advance(); // consume the operator token
                Expression right = ParsePrimary();
                left = new BinaryExpression(op, left, right, SpanFrom(left, right));
            }

            return left;
        }

        /// <summary>
        /// Parse a primary expression: an integer literal or a parenthesised subexpression
        /// (the only way to regroup, given no precedence). The parentheses are structural
        /// only — the inner expression is returned directly, since its own tree already
        /// captures the grouping.
        /// </summary>
        private Expression ParsePrimary()
        {
            Token token = Peek();
            switch (token.Kind)
            {
                case TokenKind.IntLiteral:
                    Advance();
                    // Parse into a long so ordinary literals never overflow here; the 8-bit
                    // exit-code range check is a later pass's job. Only a literal too big
                    // for a long is rejected at this stage.
                    if (!long.TryParse(token.Text, out long value))
                    {
                        throw Error(token, $"integer literal '{token.Text}' is too large");
                    }
                    return new IntLiteral(value, token.Span);

                case TokenKind.LParen:
                    Advance();
                    Expression inner = ParseExpr();
                    Expect(TokenKind.RParen, "')' to close the group");
                    return inner;

                default:
                    throw Error(token, "expected an expression");
            }
        }

        // --- Recovery ---

        /// <summary>
        /// Skip tokens after a top-level error until the start of the next declaration
        /// (<c>extern</c>/<c>fn</c>) or end-of-file, so the outer loop can resume on a
        /// fresh declaration instead of re-erroring on the same bad token. The
        /// declaration-start set is centralised here for the whole parser to extend as the
        /// grammar grows.
        /// </summary>
        private void Synchronize()
        {
            while (!AtEnd && !Check(TokenKind.Extern) && !Check(TokenKind.Fn))
            {
                Advance();
            }
        }

        /// <summary>
        /// Skip tokens after a statement error to the block's closing brace (or
        /// end-of-file). Stage 1 blocks have no statement separators, so the brace is the
        /// only reliable resync point; this is the one place to teach finer statement-level
        /// recovery once separators/keywords exist.
        /// </summary>
        private void SynchronizeStatement()
        {
            while (!Check(TokenKind.RBrace) && !AtEnd)
            {
                Advance();
            }
        }

        // --- Token cursor primitives ---

        /// <summary>True once the cursor sits on the end-of-file terminator.</summary>
        private bool AtEnd => Peek().Kind == TokenKind.EndOfFile;

        /// <summary>The current token without consuming it.</summary>
        private Token Peek() => _tokens[_position];

        /// <summary>
        /// Consume and return the current token, advancing the cursor. End-of-file is an
        /// immovable terminator — the cursor never advances past it — so callers can
        /// <see cref="Peek"/> safely without a bounds check.
        /// </summary>
        private Token Advance()
        {
            Token current = _tokens[_position];
            if (current.Kind != TokenKind.EndOfFile)
            {
                _position++;
            }
            return current;
        }

        /// <summary>True when the current token is of <paramref name="kind"/>.</summary>
        private bool Check(TokenKind kind) => Peek().Kind == kind;

        /// <summary>
        /// Consume the current token and return true if it is of <paramref name="kind"/>;
        /// otherwise leave it in place and return false. The optional-consume counterpart to
        /// <see cref="Expect"/>, used to drive the comma-separated parameter/argument loops.
        /// </summary>
        private bool Match(TokenKind kind)
        {
            if (Check(kind))
            {
                Advance();
                return true;
            }

            return false;
        }

        /// <summary>
        /// Consume the current token if it is of <paramref name="kind"/>; otherwise record
        /// a diagnostic pointing at it and unwind via <see cref="ParseError"/> to the
        /// nearest recovery boundary.
        /// </summary>
        /// <param name="kind">The token kind required here.</param>
        /// <param name="expected">
        /// A human phrase naming what was expected, e.g. <c>"'(' after the callee"</c>,
        /// spliced into the "expected …" diagnostic message.
        /// </param>
        private Token Expect(TokenKind kind, string expected)
        {
            if (Check(kind))
            {
                return Advance();
            }

            throw Error(Peek(), $"expected {expected}");
        }

        /// <summary>
        /// Record a located diagnostic at <paramref name="at"/> and return a
        /// <see cref="ParseError"/> for the caller to <c>throw</c>. Returning (rather than
        /// throwing) keeps the raise site an explicit <c>throw Error(...)</c>, which the C#
        /// compiler recognises as a terminating statement.
        /// </summary>
        private ParseError Error(Token at, string message)
        {
            _diagnostics.Add(Diagnostic.Error(at.Span, message));
            return new ParseError();
        }

        // --- Static helpers ---

        /// <summary>
        /// Map an operator <see cref="TokenKind"/> to its <see cref="BinaryOperator"/>,
        /// returning false for any non-operator token (which ends the
        /// <see cref="ParseExpr"/> loop).
        /// </summary>
        private static bool TryBinaryOperator(TokenKind kind, out BinaryOperator op)
        {
            switch (kind)
            {
                case TokenKind.Plus: op = BinaryOperator.Add; return true;
                case TokenKind.Minus: op = BinaryOperator.Subtract; return true;
                case TokenKind.Star: op = BinaryOperator.Multiply; return true;
                case TokenKind.Slash: op = BinaryOperator.Divide; return true;
                default: op = default; return false;
            }
        }

        /// <summary>
        /// True when <paramref name="kind"/> is one of the built-in type keywords
        /// (<c>i8 … u64 void</c>) — any of which may open a type reference.
        /// </summary>
        private static bool IsBuiltInTypeKind(TokenKind kind) => kind switch
        {
            TokenKind.I8 or TokenKind.I16 or TokenKind.I32 or TokenKind.I64
                or TokenKind.U8 or TokenKind.U16 or TokenKind.U32 or TokenKind.U64
                or TokenKind.Void => true,
            _ => false,
        };

        /// <summary>Span covering two tokens, from the start of the first to the end of the last.</summary>
        private static SourceSpan SpanFrom(Token start, Token end) => SpanFrom(start.Span, end.Span);

        /// <summary>Span covering two nodes, from the start of the first to the end of the last.</summary>
        private static SourceSpan SpanFrom(AstNode start, AstNode end) => SpanFrom(start.Span, end.Span);

        /// <summary>
        /// Combine two spans into one running from the start of <paramref name="start"/> to
        /// the end of <paramref name="end"/>. Line/column/offset come from the first; the
        /// length reaches to the end of the second.
        /// </summary>
        private static SourceSpan SpanFrom(SourceSpan start, SourceSpan end)
        {
            int length = end.Offset + end.Length - start.Offset;
            return new SourceSpan(start.Line, start.Column, start.Offset, length);
        }
    }
}
