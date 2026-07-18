using LLVMSharp.Interop;
using Suru.Compiler.CodeGen;

namespace Suru.Compiler.Emit;

public static class ObjectEmitter
{

    public const string TargetTriple = "x86_64-unknown-linux-gnu";

    private static readonly Lock InitializationLock = new();
    private static bool _targetInitialized;

    /// <summary>
    /// Emit <paramref name="codegen"/>'s module as an ELF object file at <paramref name="objectPath"/>,
    /// overwriting any existing file. The parent directory must already exist (see
    /// <see cref="BuildLayout.EnsureBuildDirectory"/>) — LLVM does not create it.
    ///
    /// <para>Emission also stamps the module with the target triple and the target machine's data
    /// layout. That is not bookkeeping: the backend needs both to lay out and mangle correctly, and
    /// stamping them means IR later dumped from this module is self-describing.</para>
    /// </summary>
    /// <param name="codegen">The codegen result owning the module to emit (its module is mutated:
    /// the triple and data layout are set on it).</param>
    /// <param name="objectPath">Absolute or relative path of the object file to write.</param>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="codegen"/> is null.</exception>
    /// <exception cref="ArgumentException">Thrown when <paramref name="objectPath"/> is blank.</exception>
    /// <exception cref="InvalidOperationException">
    /// Thrown when LLVM cannot resolve the target or refuses to emit — a broken precondition
    /// (malformed module) or a broken toolchain, never a bad Suru program.
    /// </exception>
    public static void EmitObject(CodegenResult codegen, string objectPath)
    {
        ArgumentNullException.ThrowIfNull(codegen);
        ArgumentException.ThrowIfNullOrWhiteSpace(objectPath);

        EnsureTargetInitialized();

        if (!LLVMTargetRef.TryGetTargetFromTriple(TargetTriple, out LLVMTargetRef target, out string lookupError))
        {
            throw new InvalidOperationException(
                $"emit: LLVM has no backend registered for '{TargetTriple}': {lookupError}");
        }

        // "generic" CPU with no extra features. PIC relocation is required, not a preference — clang links
        // position-independent executables by default on Ubuntu, and a non-PIC object would be
        // rejected at link time with a relocation error.
        LLVMTargetMachineRef machine = target.CreateTargetMachine(
            TargetTriple,
            cpu: "generic",
            features: "",
            LLVMCodeGenOptLevel.LLVMCodeGenLevelDefault,
            LLVMRelocMode.LLVMRelocPIC,
            LLVMCodeModel.LLVMCodeModelDefault);

        try
        {
            LLVMModuleRef module = codegen.Module;
            module.Target = TargetTriple;
            module.DataLayout = DescribeDataLayout(machine);

            if (!machine.TryEmitToFile(module, objectPath, LLVMCodeGenFileType.LLVMObjectFile, out string emitError))
            {
                throw new InvalidOperationException(
                    $"emit: LLVM failed to write the object file '{objectPath}': {emitError}");
            }
        }
        finally
        {
            // The target machine is an unmanaged LLVM resource, but LLVMSharp's handle wrapper is
            // not IDisposable, so it is released through the raw C entry point.
            unsafe
            {
                LLVM.DisposeTargetMachine(machine);
            }
        }
    }

    /// <summary>
    /// Get a target machine's data layout as the string LLVM's module parser expects (e.g.
    /// <c>e-m:e-p270:32:32-…-S128</c>).
    ///
    /// <para>This drops to the raw C entry points on purpose. The obvious-looking
    /// <c>machine.CreateTargetDataLayout().ToString()</c> does <b>not</b> work: LLVMSharp's handle
    /// wrapper inherits a debugging <c>ToString</c> that renders as
    /// <c>"LLVMTargetDataRef: 5D22FF680EB0"</c>, and feeding that to the module aborts the process
    /// with <c>LLVM ERROR: unknown specifier 'L'</c>. The layout text only comes out of
    /// <c>LLVMCopyStringRepOfTargetData</c>.</para>
    /// </summary>
    private static unsafe string DescribeDataLayout(LLVMTargetMachineRef machine)
    {
        LLVMOpaqueTargetData* targetData = LLVM.CreateTargetDataLayout(machine);
        try
        {
            // The C API hands back a malloc'd string that the caller owns; copy it into managed
            // memory and give it straight back to LLVM's allocator.
            sbyte* layout = LLVM.CopyStringRepOfTargetData(targetData);
            try
            {
                return new string(layout);
            }
            finally
            {
                LLVM.DisposeMessage(layout);
            }
        }
        finally
        {
            LLVM.DisposeTargetData(targetData);
        }
    }

    /// <summary>
    /// Register the X86 backend with LLVM's global target registry, exactly once per process.
    ///
    /// <para>The registry is process-global mutable state (unlike the per-call
    /// <c>LLVMContextRef</c> codegen creates), so this is guarded by a lock: xUnit runs test
    /// classes in parallel, and two threads registering concurrently would race. Only X86 is
    /// registered — <c>InitializeAllTargets</c> would pull in every backend for no benefit while
    /// Stage 1 emits for one triple.</para>
    /// </summary>
    private static void EnsureTargetInitialized()
    {
        lock (InitializationLock)
        {
            if (_targetInitialized)
            {
                return;
            }

            LLVM.InitializeX86TargetInfo();  // registers the target so it can be found by triple
            LLVM.InitializeX86Target();      // the backend itself
            LLVM.InitializeX86TargetMC();    // machine-code layer (instruction/register info)
            LLVM.InitializeX86AsmPrinter();  // required to emit, even for object output

            _targetInitialized = true;
        }
    }
}
