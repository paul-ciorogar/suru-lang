namespace Suru.Compiler.Parsing;

/// <summary>
/// The base of every abstract syntax tree node.
/// </summary>
/// <param name="span">Where in the source this node's text is.</param>
public abstract class AstNode(SourceSpan span)
{
    /// <summary>Where in the source this node's text is.</summary>
    public SourceSpan Span { get; } = span;

    /// <summary>
    /// The type the semantic pass resolves for this node, or <c>null</c> before that
    /// pass runs. 
    /// </summary>
    public SuruType? ResolvedType { get; set; }
}

/// <summary>
/// The base of every value-producing node — an arithmetic expression, a literal, an
/// identifier reference, or a call. Grouped under one type so the parser and later
/// passes can pass "any expression" around uniformly (e.g. the operands of a
/// <see cref="BinaryExpression"/> or the argument of a <see cref="CallExpression"/>).
/// </summary>
/// <param name="span">Where in the source this expression's text is.</param>
public abstract class Expression(SourceSpan span) : AstNode(span);

/// <summary>
/// The base of every node that appears in statement position — the elements of a
/// <see cref="Block"/>. A statement is executed for its effect, not for a value;
/// grouping them under one type lets a block hold "any statement" uniformly.
/// </summary>
/// <param name="span">Where in the source this statement's text is.</param>
public abstract class Statement(SourceSpan span) : AstNode(span);

/// <summary>
/// A binary operator. All four share a single precedence level and evaluate
/// strictly left-to-right — there is no math precedence in Suru — so this enum only
/// names the operation; associativity is fixed by how the parser nests the tree.
/// </summary>
public enum BinaryOperator
{
    /// <summary>Addition, <c>+</c>.</summary>
    Add,

    /// <summary>Subtraction, <c>-</c>.</summary>
    Subtract,

    /// <summary>Multiplication, <c>*</c>.</summary>
    Multiply,

    /// <summary>Division, <c>/</c>.</summary>
    Divide,
}

/// <summary>
/// An integer literal, e.g. <c>1</c> or <c>42</c>.
///
/// The value is stored as a <see cref="long"/> so a parsed literal cannot overflow
/// before the downstream 8-bit exit-code constraint is checked; range checking is a
/// later pass's job, not this node's.
/// </summary>
/// <param name="value">The literal's integer value.</param>
/// <param name="span">Where in the source the literal is.</param>
public sealed class IntLiteral(long value, SourceSpan span) : Expression(span)
{
    /// <summary>The literal's integer value.</summary>
    public long Value { get; } = value;
}

/// <summary>
/// A reference to a name in an expression position — for now, the callee of a
/// <see cref="CallExpression"/>. Resolution of the name to a declaration is the
/// semantic pass's job; this node only records the name and where it was written.
/// </summary>
/// <param name="name">The referenced identifier's spelling.</param>
/// <param name="span">Where in the source the identifier is.</param>
public sealed class IdentifierExpression(string name, SourceSpan span) : Expression(span)
{
    /// <summary>The referenced identifier's spelling.</summary>
    public string Name { get; } = name;
}

/// <summary>
/// A binary arithmetic expression, <c>Left Operator Right</c>. Because Suru has no
/// operator precedence, the parser builds these left-associatively: <c>1 + 2 * 3</c>
/// nests as <c>((1 + 2) * 3)</c>, with the left operand itself a
/// <see cref="BinaryExpression"/>.
/// </summary>
/// <param name="op">Which arithmetic operation this expression performs.</param>
/// <param name="left">The left operand.</param>
/// <param name="right">The right operand.</param>
/// <param name="span">Where in the source the whole expression is.</param>
public sealed class BinaryExpression(
    BinaryOperator op, Expression left, Expression right, SourceSpan span) : Expression(span)
{
    /// <summary>Which arithmetic operation this expression performs.</summary>
    public BinaryOperator Operator { get; } = op;

    /// <summary>The left operand.</summary>
    public Expression Left { get; } = left;

    /// <summary>The right operand.</summary>
    public Expression Right { get; } = right;
}

/// <summary>
/// A call of a named function with a single argument, e.g. <c>exit(9)</c>.
///
/// The callee is kept as an <see cref="IdentifierExpression"/> rather than a bare
/// string so it carries its own <see cref="SourceSpan"/> — the semantic pass reports
/// "unknown callee" pointing at the name, not the whole call.
/// </summary>
/// <param name="callee">The name being called.</param>
/// <param name="argument">The single argument expression.</param>
/// <param name="span">Where in the source the whole call is.</param>
public sealed class CallExpression(
    IdentifierExpression callee, Expression argument, SourceSpan span) : Expression(span)
{
    /// <summary>The name being called.</summary>
    public IdentifierExpression Callee { get; } = callee;

    /// <summary>The single argument expression.</summary>
    public Expression Argument { get; } = argument;
}

/// <summary>
/// An expression used in statement position, e.g. a bare call <c>exit(9)</c> on its
/// own line. The wrapped expression is evaluated for its effect and its value is
/// discarded; the wrapper is what lets a <see cref="Block"/> hold it alongside other
/// statements.
/// </summary>
/// <param name="expression">The expression evaluated for its effect.</param>
/// <param name="span">Where in the source the statement is.</param>
public sealed class ExpressionStatement(Expression expression, SourceSpan span) : Statement(span)
{
    /// <summary>The expression evaluated for its effect.</summary>
    public Expression Expression { get; } = expression;
}

/// <summary>
/// A brace-delimited block of statements.
/// </summary>
/// <param name="statements">The statements in the block, in source order.</param>
/// <param name="span">Where in the source the block (braces included) is.</param>
public sealed class Block(IReadOnlyList<Statement> statements, SourceSpan span) : AstNode(span)
{
    public IReadOnlyList<Statement> Statements { get; } = statements;
}

/// <summary>
/// An <c>extern IDENT</c> declaration: it brings a C function into scope by name so the
/// program can call it.
/// </summary>
/// <param name="name">The declared external name.</param>
/// <param name="span">Where in the source the declaration is.</param>
public sealed class ExternDeclaration(string name, SourceSpan span) : AstNode(span)
{
    public string Name { get; } = name;
}

/// <summary>
/// A function definition, <c>fn IDENT() { … }</c>.
/// </summary>
/// <param name="name">The function's name.</param>
/// <param name="body">The function's body block.</param>
/// <param name="span">Where in the source the whole definition is.</param>
public sealed class FunctionDefinition(string name, Block body, SourceSpan span) : AstNode(span)
{
    public string Name { get; } = name;

    public Block Body { get; } = body;
}

/// <summary>
/// The result of parsing — a whole Suru program: its extern declarations and its
/// function definitions. A program may define any number of functions; the one named
/// <see cref="EntryPointName"/> is its entry point, surfaced as <see cref="Main"/>.
/// </summary>
/// <param name="externs">The program's extern declarations, in source order.</param>
/// <param name="functions">Every function the program defines, in source order.</param>
/// <param name="main">The entry-point function (the one named <see cref="EntryPointName"/>).</param>
public sealed class SuruProgram(
    IReadOnlyList<ExternDeclaration> externs,
    IReadOnlyList<FunctionDefinition> functions,
    FunctionDefinition main)
{
    /// <summary>The name of the entry-point function every program must define.</summary>
    public const string EntryPointName = "main";

    /// <summary>The program's extern declarations.</summary>
    public IReadOnlyList<ExternDeclaration> Externs { get; } = externs;

    /// <summary>Every function the program defines, in source order.</summary>
    public IReadOnlyList<FunctionDefinition> Functions { get; } = functions;

    /// <summary>The program's entry point — the function named <see cref="EntryPointName"/>.</summary>
    public FunctionDefinition Main { get; } = main;
}
