// Suru.Cli — entry point for the `suru` command-line tool.
//
//   suru run   <file.suru>   compile and run a Suru program
//   suru build               build the current project/solution
//   suru test                run the test suite
//   suru ir    <file.suru>   dump the LLVM IR for a Suru source file

using Suru.Compiler;

return Cli.Run(args);

/// <summary>
/// The CLI dispatcher. Kept as a static class (rather than top-level-only code)
/// so the individual pieces are named and unit-testable later.
/// </summary>
internal static class Cli
{
    /// <summary>The command completed successfully.</summary>
    public const int Success = 0;

    /// <summary>A usage or runtime error (bad arguments, failed compile).</summary>
    public const int Failure = 1;

    /// <summary>The requested subcommand is not recognized.</summary>
    public const int UnknownCommandCode = 2;

    /// <summary>Parse <paramref name="args"/> and dispatch to a subcommand.</summary>
    /// <returns>A process exit code.</returns>
    public static int Run(string[] args)
    {
        // No subcommand, or an explicit help request, prints usage. A bare
        // invocation is a usage error (exit 1); an explicit --help is success.
        if (args.Length == 0)
        {
            PrintUsage(Console.Error);
            return Failure;
        }

        string command = args[0];
        // Everything after the subcommand is that command's own arguments.
        string[] rest = args[1..];

        return command switch
        {
            "--help" or "-h" or "help" => Help(),
            "run" => RunCommand(rest),
            "build" => BuildCommand(rest),
            "test" => TestCommand(rest),
            "ir" => IrCommand(rest),
            _ => UnknownCommand(command),
        };
    }

    /// <summary>`suru run &lt;file.suru&gt;` — compile and run a program.</summary>
    private static int RunCommand(string[] args)
    {
        if (!TryGetSourcePath(args, "run", out string sourcePath))
        {
            return Failure;
        }

        CompileResult result = Suru.Compiler.Compiler.Compile(sourcePath);
        // TODO run
        Console.WriteLine(result.Message);
        return result.Succeeded ? Success : Failure;
    }

    /// <summary>`suru ir &lt;file.suru&gt;` — dump LLVM IR for a source file.</summary>
    private static int IrCommand(string[] args)
    {
        if (!TryGetSourcePath(args, "ir", out string sourcePath))
        {
            return Failure;
        }

        CompileResult result = Suru.Compiler.Compiler.Compile(sourcePath);
        // TODO IR emission 
        Console.WriteLine(result.Message);
        return result.Succeeded ? Success : Failure;
    }

    /// <summary>`suru build` — build the project. Stub until the build pipeline lands.</summary>
    private static int BuildCommand(string[] args)
    {
        Console.WriteLine("suru build: not yet implemented.");
        return Success;
    }

    /// <summary>`suru test` — run the test suite.</summary>
    private static int TestCommand(string[] args)
    {
        Console.WriteLine("suru test: not yet implemented.");
        return Success;
    }

    /// <summary>Print full usage and exit successfully (explicit help request).</summary>
    private static int Help()
    {
        PrintUsage(Console.Out);
        return Success;
    }

    /// <summary>Report an unrecognized subcommand and exit non-zero.</summary>
    private static int UnknownCommand(string command)
    {
        Console.Error.WriteLine($"suru: unknown command '{command}'.");
        PrintUsage(Console.Error);
        return UnknownCommandCode;
    }

    /// <summary>
    /// Extract the required single source-path argument for `run`/`ir`. Writes a
    /// usage error to stderr and returns false when it is missing.
    /// </summary>
    private static bool TryGetSourcePath(string[] args, string command, out string sourcePath)
    {
        if (args.Length < 1)
        {
            Console.Error.WriteLine($"suru {command}: expected a .suru source file path.");
            sourcePath = string.Empty;
            return false;
        }

        sourcePath = args[0];
        return true;
    }

    /// <summary>Write the usage/help text to the given writer.</summary>
    private static void PrintUsage(TextWriter w)
    {
        w.WriteLine("suru — the Suru language toolchain");
        w.WriteLine();
        w.WriteLine("Usage: suru <command> [arguments]");
        w.WriteLine();
        w.WriteLine("Commands:");
        w.WriteLine("  run   <file.suru>   Compile and run a Suru program");
        w.WriteLine("  build               Build the current project");
        w.WriteLine("  test                Run the test suite");
        w.WriteLine("  ir    <file.suru>   Print the LLVM IR for a source file");
        w.WriteLine();
        w.WriteLine("  -h, --help          Show this help");
    }
}
