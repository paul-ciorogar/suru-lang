using LLVMSharp.Interop;
using Suru.Compiler.Parsing;

namespace Suru.Compiler.CodeGen;

/// <summary>
/// The LLVM codegen pass: the fourth stage of the pipeline, run on a program that has already
/// passed <c>SemanticAnalyzer.Analyze</c>. It lowers the <see cref="SuruProgram"/> AST into an
/// in-memory LLVM module 
///
/// <para><b>Precondition:</b> the program is semantically valid and already analysed.
/// </summary>
public static class CodeGenerator
{
    /// <summary>
    /// Lower a semantically-analysed program into an LLVM module.
    /// </summary>
    /// <param name="program">The analysed program to lower (its nodes are read, not mutated).</param>
    /// <returns>
    /// A <see cref="CodegenResult"/> owning the generated module and its context; the caller
    /// disposes it (both are unmanaged LLVM resources).
    /// </returns>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="program"/> is null.</exception>
    public static CodegenResult Generate(SuruProgram program)
    {
        ArgumentNullException.ThrowIfNull(program);

        LLVMContextRef context = LLVMContextRef.Create();
        LLVMModuleRef module = context.CreateModuleWithName("suru");

        // Declare every extern up front and index it by name (value + function type). LLVM 20 uses
        // opaque pointers, so the call site needs the function type back — hence we keep both.
        Dictionary<string, (LLVMValueRef Function, LLVMTypeRef Type)> externs = new(StringComparer.Ordinal);
        foreach (ExternDeclaration declaration in program.Externs)
        {
            LLVMTypeRef[] parameterTypes = new LLVMTypeRef[declaration.Parameters.Count];
            for (int i = 0; i < declaration.Parameters.Count; i++)
            {
                parameterTypes[i] = MapType(declaration.Parameters[i], context);
            }

            LLVMTypeRef functionType =
                LLVMTypeRef.CreateFunction(MapType(declaration.ReturnType, context), parameterTypes, IsVarArg: false);

            // AddFunction with no body gives external linkage — i.e. a `declare`.
            externs[declaration.Name] = (module.AddFunction(declaration.Name, functionType), functionType);
        }

        // Stage 1 compiles exactly one function — the entry point, `i32 @main()`. Its return value
        // is unused at runtime (the process observes the `exit` argument via its exit code), but a
        // real signature keeps the function well-formed.
        LLVMTypeRef i32 = context.Int32Type;
        LLVMValueRef main = module.AddFunction(
            SuruProgram.EntryPointName,
            LLVMTypeRef.CreateFunction(i32, Array.Empty<LLVMTypeRef>(), IsVarArg: false));

        LLVMBasicBlockRef entry = context.AppendBasicBlock(main, "entry");
        LLVMBuilderRef builder = context.CreateBuilder();
        builder.PositionAtEnd(entry);

        foreach (Statement statement in program.Main.Body.Statements)
        {
            LowerStatement(statement, builder, externs, i32);
        }

        // Terminate main with `ret i32 0` rather than the `unreachable` the task text names: `exit`
        // ends the process before control returns, so the exit code is unaffected, and a real `ret`
        // keeps main valid for the general case (several statements, value-returning externs).
        builder.BuildRet(LLVMValueRef.CreateConstInt(i32, 0, SignExtend: false));

        builder.Dispose();
        return new CodegenResult(context, module);
    }

    /// <summary>
    /// Lower one statement. Stage 1 statements are bare calls (an <see cref="ExpressionStatement"/>
    /// wrapping a <see cref="CallExpression"/>); anything else is a broken precondition (the
    /// semantic pass would have rejected it), reported by throwing rather than emitting bad IR.
    /// </summary>
    private static void LowerStatement(
        Statement statement,
        LLVMBuilderRef builder,
        IReadOnlyDictionary<string, (LLVMValueRef Function, LLVMTypeRef Type)> externs,
        LLVMTypeRef i32)
    {
        if (statement is not ExpressionStatement { Expression: CallExpression call })
        {
            throw new InvalidOperationException(
                $"codegen expects each statement to be a call; got {statement.GetType().Name}.");
        }

        if (!externs.TryGetValue(call.Callee.Name, out (LLVMValueRef Function, LLVMTypeRef Type) callee))
        {
            throw new InvalidOperationException(
                $"codegen: call to undeclared function '{call.Callee.Name}' (should have been caught by the semantic pass).");
        }

        LLVMValueRef[] arguments = new LLVMValueRef[call.Arguments.Count];
        for (int i = 0; i < call.Arguments.Count; i++)
        {
            arguments[i] = LowerExpression(call.Arguments[i], builder, i32);
        }

        // Empty result name — a void call produces no named value (and even a valued Stage 1 call
        // is discarded in statement position).
        builder.BuildCall2(callee.Type, callee.Function, arguments, "");
    }

    /// <summary>
    /// Lower a value expression to the SSA value it computes. The parser has already baked Suru's
    /// no-precedence, left-to-right grouping into the tree shape (<c>1 + 2 * 3</c> is
    /// <c>((1 + 2) * 3)</c>), so this is a straight post-order walk with no precedence handling.
    /// </summary>
    private static LLVMValueRef LowerExpression(Expression expression, LLVMBuilderRef builder, LLVMTypeRef i32)
    {
        switch (expression)
        {
            case IntLiteral literal:
                // Value fits i32 (the 8-bit exit-code constraint is checked upstream); it is
                // non-negative here, so the sign-extend flag is immaterial.
                return LLVMValueRef.CreateConstInt(i32, unchecked((ulong)literal.Value), SignExtend: false);

            case BinaryExpression binary:
                LLVMValueRef left = LowerExpression(binary.Left, builder, i32);
                LLVMValueRef right = LowerExpression(binary.Right, builder, i32);
                return binary.Operator switch
                {
                    BinaryOperator.Add => builder.BuildAdd(left, right, "add"),
                    BinaryOperator.Subtract => builder.BuildSub(left, right, "sub"),
                    BinaryOperator.Multiply => builder.BuildMul(left, right, "mul"),
                    // Signed division — Stage 1 integers are signed i32.
                    BinaryOperator.Divide => builder.BuildSDiv(left, right, "div"),
                    _ => throw new InvalidOperationException($"codegen: unknown operator {binary.Operator}."),
                };

            default:
                // Identifiers and calls are names/effects, not i32 values, and cannot appear as an
                // argument in Stage 1; the semantic pass guarantees this.
                throw new InvalidOperationException(
                    $"codegen expects an integer expression; got {expression.GetType().Name}.");
        }
    }

    /// <summary>
    /// Map a resolved Suru type reference to its LLVM type. Widths only — LLVM integers are
    /// sign-agnostic, so <c>i32</c> and <c>u32</c> both become a 32-bit integer. Reads the slot the
    /// semantic pass stamped, falling back to re-resolving the spelling defensively.
    /// </summary>
    private static LLVMTypeRef MapType(TypeReference reference, LLVMContextRef context)
    {
        SuruType? type = reference.ResolvedType;
        if (type is null && !SuruType.TryResolve(reference.Name, out type))
        {
            throw new InvalidOperationException($"codegen: unresolved type '{reference.Name}'.");
        }

        return type!.Name switch
        {
            "i8" or "u8" => context.Int8Type,
            "i16" or "u16" => context.Int16Type,
            "i32" or "u32" => context.Int32Type,
            "i64" or "u64" => context.Int64Type,
            "void" => context.VoidType,
            _ => throw new InvalidOperationException($"codegen: no LLVM mapping for type '{type.Name}'."),
        };
    }
}

/// <summary>
/// The outcome of <see cref="CodeGenerator.Generate"/>: the generated LLVM module plus the context
/// that owns it. Both are unmanaged LLVM resources, so this is <see cref="IDisposable"/>; the
/// context is kept alive alongside the module because later stages (Task 6's target machine and
/// object emission) need it. Dispose the module before the context.
/// </summary>
public sealed class CodegenResult : IDisposable
{
    private readonly LLVMContextRef _context;
    private bool _disposed;

    internal CodegenResult(LLVMContextRef context, LLVMModuleRef module)
    {
        _context = context;
        Module = module;
    }

    /// <summary>The generated module. Consumed by object emission and the <c>ir</c> subcommand.</summary>
    public LLVMModuleRef Module { get; }

    /// <summary>
    /// Render the module as LLVM IR text — what the <c>ir</c> subcommand dumps and what codegen
    /// tests assert on.
    /// </summary>
    public string ToIrString() => Module.PrintToString();

    /// <summary>Dispose the module and its context (module first).</summary>
    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        Module.Dispose();
        _context.Dispose();
        _disposed = true;
    }
}
