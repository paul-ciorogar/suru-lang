using System.ComponentModel;
using System.Diagnostics;
using System.Text;

namespace Suru.Compiler.Emit;

/// <summary>
/// The link pass: the final stage of the pipeline. It turns the relocatable object
/// <see cref="ObjectEmitter"/> wrote into a runnable native ELF by invoking <b>clang</b> as a child
/// process — the repo's first use of <see cref="Process"/>.
///
/// <para><b>Why shell out rather than call a library?</b> Linking needs far more than a linker:
/// the C runtime startup files (<c>crt1.o</c> and friends), the libc search paths, and the dynamic
/// loader path all vary per distribution. <c>clang</c> is the driver that already knows all of
/// them. Reimplementing that knowledge to call <c>ld.lld</c> directly would buy nothing and break
/// on the next base image.</para>
///
/// <para><b>Why a result rather than an exception?</b> Unlike codegen and emission, this stage
/// <i>can</i> fail because of a bad Suru program: an <c>extern fn nosuchthing(i32)</c> compiles and
/// emits cleanly, then fails here with an undefined-symbol error. That is a user mistake, so it is
/// reported as a <see cref="LinkResult"/> to be shown, not thrown as if the compiler broke.</para>
/// </summary>
public static class Linker
{
    /// <summary>
    /// The link driver, resolved from <c>PATH</c> (the dev image puts LLVM's unversioned
    /// <c>clang</c> on it). Not an absolute path: the container and any future host toolchain
    /// install it in different places.
    /// </summary>
    public const string LinkerCommand = "clang";

    /// <summary>
    /// Link an object file into a native executable, overwriting any existing file at
    /// <paramref name="executablePath"/>. The parent directory must already exist (see
    /// <see cref="BuildLayout.EnsureBuildDirectory"/>).
    ///
    /// <para>The object is linked against the default C runtime with no extra flags — that is what
    /// supplies <c>_start</c> (which calls the module's <c>main</c>) and libc, where Stage 1's
    /// <c>extern exit</c> resolves.</para>
    /// </summary>
    /// <param name="objectPath">Path to the object file to link.</param>
    /// <param name="executablePath">Path of the executable to produce.</param>
    /// <returns>
    /// A <see cref="LinkResult"/> reporting success plus whatever the driver printed. Failure is a
    /// returned result, not an exception — including <c>clang</c> being absent from <c>PATH</c>.
    /// </returns>
    /// <exception cref="ArgumentException">Thrown when either path is blank.</exception>
    public static LinkResult Link(string objectPath, string executablePath)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(objectPath);
        ArgumentException.ThrowIfNullOrWhiteSpace(executablePath);

        ProcessStartInfo startInfo = new()
        {
            FileName = LinkerCommand,
            // Arguments are added one by one rather than as a single command line: .NET quotes and
            // escapes each, so a path containing a space cannot be split into two arguments.
            ArgumentList = { objectPath, "-o", executablePath },
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
        };

        using Process? process = StartLinker(startInfo, out string? startFailure);
        if (process is null)
        {
            return LinkResult.Failure(startFailure!);
        }

        // Read both streams to completion before waiting on exit: a process that fills a redirected
        // pipe's buffer blocks forever if nobody drains it, and reading them sequentially would
        // deadlock on whichever stream is written second.
        Task<string> standardOutput = process.StandardOutput.ReadToEndAsync();
        Task<string> standardError = process.StandardError.ReadToEndAsync();
        process.WaitForExit();

        string output = CombineOutput(standardOutput.Result, standardError.Result);

        if (process.ExitCode != 0)
        {
            return LinkResult.Failure(
                $"{LinkerCommand} failed with exit code {process.ExitCode} while linking '{objectPath}'."
                + (output.Length > 0 ? Environment.NewLine + output : string.Empty));
        }

        return LinkResult.Success(executablePath, output);
    }

    /// <summary>
    /// Start the driver, translating "the executable isn't there" into a message instead of an
    /// exception — a missing <c>clang</c> is an environment problem the caller must report, and it
    /// is the one failure that surfaces as <see cref="Win32Exception"/> rather than an exit code.
    /// </summary>
    private static Process? StartLinker(ProcessStartInfo startInfo, out string? failure)
    {
        try
        {
            Process? process = Process.Start(startInfo);
            if (process is null)
            {
                failure = $"Could not start the linker ('{LinkerCommand}').";
                return null;
            }

            failure = null;
            return process;
        }
        catch (Win32Exception exception)
        {
            failure =
                $"Could not run the linker ('{LinkerCommand}'): {exception.Message}. "
                + "Is a C toolchain on PATH? The Suru dev container provides one.";
            return null;
        }
    }

    /// <summary>Join the driver's two streams into one block, dropping whichever is empty.</summary>
    private static string CombineOutput(string standardOutput, string standardError)
    {
        StringBuilder combined = new();
        foreach (string stream in new[] { standardOutput, standardError })
        {
            string trimmed = stream.Trim();
            if (trimmed.Length == 0)
            {
                continue;
            }

            if (combined.Length > 0)
            {
                combined.AppendLine();
            }

            combined.Append(trimmed);
        }

        return combined.ToString();
    }
}

/// <summary>
/// The outcome of <see cref="Linker.Link"/>.
/// </summary>
/// <param name="Succeeded">True if the driver exited 0 and the executable was produced.</param>
/// <param name="ExecutablePath">Path of the linked executable; null when linking failed.</param>
/// <param name="Message">
/// What the driver printed, plus a description of the failure when it failed. May be empty on
/// success — a clean link is silent.
/// </param>
public readonly record struct LinkResult(bool Succeeded, string? ExecutablePath, string Message)
{
    /// <summary>Construct the success outcome for a produced executable.</summary>
    public static LinkResult Success(string executablePath, string message) =>
        new(true, executablePath, message);

    /// <summary>Construct a failure outcome — no executable, and a message explaining why.</summary>
    public static LinkResult Failure(string message) => new(false, null, message);
}
