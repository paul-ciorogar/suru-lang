using System.Collections;

namespace Suru.Compiler.Lexing;

/// <summary>
/// An append-only collection of <see cref="Token"/>s that <em>interns</em> each
/// token's text — the lexer's output buffer.
///
/// Source is dominated by repeated user-written names: every mention of <c>exit</c>,
/// <c>main</c>, a variable, or a literal is the same short string over and over.
/// Storing a distinct <see cref="string"/> per token would allocate one small string
/// per occurrence and keep them all alive. Instead, <see cref="AddToken"/> routes the
/// text of the variable-spelling kinds (identifiers and int literals) through an
/// internal registry (<see cref="_registry"/>) so that all tokens sharing the same
/// text point at a <em>single</em> canonical string instance. That bounds string
/// allocations by the number of <em>distinct</em> lexemes, not the number of tokens,
/// and lets later passes compare identifiers by reference.
///
/// Fixed-spelling tokens — operators, brackets, keywords, end-of-file — are added via
/// <see cref="AddFixedToken"/> instead: their text is a compile-time constant, so
/// there is nothing to deduplicate and they never touch the registry.

/// </summary>
public sealed class Tokens : IReadOnlyList<Token>
{
    /// <summary>The tokens in scan order.</summary>
    private readonly List<Token> _items = new();

    /// <summary>
    /// The interning pool: maps a lexeme to the one canonical string instance used
    /// for every token with that text. Key and value are the same reference; the
    /// dictionary is used as a "hash set that returns the stored element".
    ///
    /// It MUST be constructed with an ordinal comparer: identifiers are
    /// case-sensitive, and — more importantly — <see cref="StringComparer.Ordinal"/>
    /// is what enables the allocation-free <see cref="ReadOnlySpan{Char}"/> lookup in
    /// <see cref="Intern"/> (the default comparer does not support alternate lookups).
    /// </summary>
    private readonly Dictionary<string, string> _registry = new(StringComparer.Ordinal);

    /// <summary>
    /// Append an <em>interned</em> token for a kind whose text is user-written and
    /// read from the source — an <see cref="TokenKind.Identifier"/> or an
    /// <see cref="TokenKind.IntLiteral"/>. These are the lexemes worth deduplicating,
    /// because the same name or literal recurs throughout a program.
    ///
    /// Callers pass the raw source slice as a <see cref="ReadOnlySpan{Char}"/> rather
    /// than a pre-cut string: when the lexeme has been seen before, no string is
    /// allocated at all (the span is matched against the pool directly). A new string
    /// is materialised only the first time a distinct lexeme appears.
    ///
    /// Fixed-spelling tokens (operators, brackets, keywords, end-of-file) must NOT be
    /// routed here — use <see cref="AddFixedToken"/>, which skips the registry.
    /// </summary>
    /// <param name="kind">The lexical category of the token (a variable-text kind).</param>
    /// <param name="text">The token's source text as a span.</param>
    /// <param name="span">Where in the source the token is.</param>
    public void AddToken(TokenKind kind, ReadOnlySpan<char> text, SourceSpan span)
    {
        _items.Add(new TextToken(kind, Intern(text), span));
    }

    /// <summary>
    /// Append a token whose spelling is fixed by its <see cref="TokenKind"/> — an
    /// operator, bracket, keyword, or end-of-file. 
    /// </summary>
    /// <param name="kind">A fixed-text token kind.</param>
    /// <param name="span">Where in the source the token is.</param>
    public void AddFixedToken(TokenKind kind, SourceSpan span)
    {
        _items.Add(new FixedToken(kind, span));
    }

    /// <summary>
    /// Return the canonical string for <paramref name="text"/>, adding it to the pool
    /// on first sight. Uses the dictionary's <see cref="ReadOnlySpan{Char}"/> alternate
    /// lookup so an already-seen lexeme is resolved without allocating a string; only a
    /// genuinely new lexeme is copied out of the source via <see cref="ReadOnlySpan{Char}.ToString"/>.
    /// </summary>
    private string Intern(ReadOnlySpan<char> text)
    {
        Dictionary<string, string>.AlternateLookup<ReadOnlySpan<char>> lookup =
            _registry.GetAlternateLookup<ReadOnlySpan<char>>();

        if (lookup.TryGetValue(text, out string? existing))
        {
            return existing;
        }

        // First occurrence of this lexeme: materialise it once and store it as its
        // own key/value so future occurrences reuse this exact instance.
        string created = text.ToString();
        _registry[created] = created;
        return created;
    }

    /// <summary>Number of distinct interned lexemes — exposed for diagnostics/tests.</summary>
    public int DistinctTextCount => _registry.Count;

    // --- IReadOnlyList<Token>: the collection doubles as its own read model. ---

    /// <summary>The token at <paramref name="index"/> in scan order.</summary>
    public Token this[int index] => _items[index];

    /// <summary>The number of tokens collected.</summary>
    public int Count => _items.Count;

    /// <inheritdoc />
    public IEnumerator<Token> GetEnumerator() => _items.GetEnumerator();

    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();
}
