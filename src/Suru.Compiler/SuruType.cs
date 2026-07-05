namespace Suru.Compiler;

/// <summary>
/// A resolved Suru type — the semantic answer to "what is the type of this thing?".
///
/// This is an intentionally empty placeholder. The language currently only ever computes
/// 32-bit integers, so there is nothing to distinguish yet; the shape of the type system is
/// filled in with the semantic pass. It exists now so that AST nodes can carry a
/// strongly-typed, settable annotation slot (<c>ResolvedType</c>) that later passes
/// populate in place, rather than a loosely-typed <c>object?</c>.
/// </summary>
public sealed class SuruType;
