// Toolchain smoke test (throwaway — not part of the compiler).
//
// Proves that LLVMSharp's managed bindings can load the image's native libLLVM
// and call into the LLVM-C API, and that the native major matches the pin. If
// the managed/native majors disagreed, the very first P/Invoke below would throw
// a DllNotFoundException or crash — so reaching "SMOKE OK" is the proof.
//
// Run it via tools/smoke/run.sh inside the suru-dev container.

using LLVMSharp.Interop;

// The LLVM major we expect the native libLLVM to report — keep in sync with
// docs/toolchain.md / the Dockerfile.
const uint ExpectedLlvmMajor = 20;

unsafe
{
    // LLVMGetVersion is a trivial, side-effect-free LLVM-C call; invoking it at
    // all forces LLVMSharp to resolve and dlopen libLLVM.so.
    uint major = 0, minor = 0, patch = 0;
    LLVM.GetVersion(&major, &minor, &patch);
    Console.WriteLine($"libLLVM loaded via LLVMSharp; reported version {major}.{minor}.{patch}");

    // Exercise a bit more of the C API: create and dispose a context. This both
    // sanity-checks object lifecycle and makes the test fail loudly if symbols
    // beyond the version getter are somehow unresolved.
    LLVMOpaqueContext* ctx = LLVM.ContextCreate();
    if (ctx is null)
    {
        Console.Error.WriteLine("ERROR: LLVMContextCreate returned null");
        return 1;
    }
    LLVM.ContextDispose(ctx);
    Console.WriteLine("LLVMContextCreate/Dispose OK");

    // Fail if the native library is the wrong major — a mismatch the managed
    // bindings cannot catch on their own.
    if (major != ExpectedLlvmMajor)
    {
        Console.Error.WriteLine($"ERROR: expected LLVM major {ExpectedLlvmMajor}, got {major}");
        return 1;
    }
}

Console.WriteLine("SMOKE OK");
return 0;
