namespace Suru.Compiler;

/// <summary>
/// A resolved Suru type — the semantic answer to "what is the type of this thing?".
///
/// It is a singleton with a private constructor, so reference
/// identity <em>is</em> type identity — a later pass can compare
/// <c>node.ResolvedType == SuruType.Int32</c> and the semantic pass can hand the same
/// instance to every integer node. The shape grows (function types, user structs, sum
/// types) as the type system does; the singleton pattern is the seed.
/// </summary>
public sealed class SuruType
{
    // The signed and unsigned integer widths, plus void (the "no value" result type). Only
    // i32 is a produced value type in Stage 1; the rest are declarable in extern signatures.
    public static readonly SuruType Int8 = new("i8");
    public static readonly SuruType Int16 = new("i16");
    public static readonly SuruType Int32 = new("i32");
    public static readonly SuruType Int64 = new("i64");
    public static readonly SuruType UInt8 = new("u8");
    public static readonly SuruType UInt16 = new("u16");
    public static readonly SuruType UInt32 = new("u32");
    public static readonly SuruType UInt64 = new("u64");
    public static readonly SuruType Void = new("void");

    private SuruType(string name) => Name = name;

    /// <summary>The type's spelling, e.g. <c>"i32"</c>.</summary>
    public string Name { get; }

    /// <summary>
    /// Resolve a written type spelling to its singleton. This is the one place that maps a
    /// type name to a <see cref="SuruType"/>; keep it in sync with the lexer's type-keyword
    /// set (a mismatch is what the semantic pass's defensive "unknown type" diagnostic guards).
    /// </summary>
    /// <param name="name">The type spelling to resolve, e.g. <c>"i32"</c>.</param>
    /// <param name="type">The resolved singleton, or <c>null</c> when unresolved.</param>
    /// <returns>True when <paramref name="name"/> names a known type.</returns>
    public static bool TryResolve(string name, out SuruType? type)
    {
        type = name switch
        {
            "i8" => Int8,
            "i16" => Int16,
            "i32" => Int32,
            "i64" => Int64,
            "u8" => UInt8,
            "u16" => UInt16,
            "u32" => UInt32,
            "u64" => UInt64,
            "void" => Void,
            _ => null,
        };
        return type is not null;
    }

    /// <inheritdoc />
    public override string ToString() => Name;
}
