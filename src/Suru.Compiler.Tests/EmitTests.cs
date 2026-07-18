using System.Diagnostics;
using Suru.Compiler.CodeGen;
using Suru.Compiler.Emit;
using Suru.Compiler.Lexing;
using Suru.Compiler.Parsing;
using Suru.Compiler.Semantics;

namespace Suru.Compiler.Tests;

/// <summary>
/// Tests for the emit stage — <see cref="BuildLayout"/>, <see cref="ObjectEmitter"/>, and
/// <see cref="Linker"/> — the pipeline's last two passes, which turn codegen's in-memory module
/// into a native ELF on disk.
///
/// <para>These tests use the real native toolchain: LLVM's X86 backend writes real object files
/// and the linker really runs <c>clang</c>. That is deliberate and safe here — <c>scripts/test</c>
/// only ever runs inside the <c>suru-dev</c> container, where the pinned toolchain is present. (A
/// skip gate for a missing toolchain is Task 8's job, when the end-to-end test lands.)</para>
/// </summary>
public static class EmitTestHelpers
{
    /// <summary>Stage 1's example program: exits with <c>((1 + 2) * 3) = 9</c>, left-to-right.</summary>
    public const string ExampleProgram =
        "extern fn exit(i32)\n" +
        "\n" +
        "fn main() {\n" +
        "    exit(1 + 2 * 3)\n" +
        "}\n";

    /// <summary>The exit code <see cref="ExampleProgram"/> must produce — no operator precedence.</summary>
    public const int ExampleExitCode = 9;

    /// <summary>
    /// Run the front end over a source string and generate its module, asserting every upstream
    /// pass is clean — the emit stage's precondition is a valid, analysed program, so a test that
    /// fed it a broken one would be testing nothing.
    /// </summary>
    public static CodegenResult Generate(string source)
    {
        LexResult lexed = Lexer.Tokenize(source);
        Assert.Empty(lexed.Diagnostics);
        ParseResult parsed = Parser.Parse(lexed.Tokens);
        Assert.Empty(parsed.Diagnostics);
        Assert.NotNull(parsed.Program);
        SemanticResult analyzed = SemanticAnalyzer.Analyze(parsed.Program!);
        Assert.True(analyzed.Succeeded);
        return CodeGenerator.Generate(parsed.Program!);
    }

    /// <summary>
    /// True if the file begins with ELF's four magic bytes (<c>0x7F E L F</c>) — a cheap, format-level
    /// check that the emitter produced a real object rather than an empty or truncated file.
    /// </summary>
    public static bool IsElf(string path)
    {
        byte[] header = new byte[4];
        using FileStream stream = File.OpenRead(path);
        return stream.Read(header, 0, 4) == 4
            && header[0] == 0x7F && header[1] == (byte)'E' && header[2] == (byte)'L' && header[3] == (byte)'F';
    }

    /// <summary>Run a linked executable to completion and return its process exit code.</summary>
    public static int RunExecutable(string path)
    {
        using Process process = Process.Start(new ProcessStartInfo(path) { UseShellExecute = false })!;
        process.WaitForExit();
        return process.ExitCode;
    }
}

/// <summary>
/// A throwaway directory standing in for a user's source folder, deleted when the test ends. Every
/// emit test writes real files, and artifacts land beside the source by policy — so tests must own
/// a disposable "beside" rather than write into the repo.
/// </summary>
public sealed class TempWorkspace : IDisposable
{
    public TempWorkspace()
    {
        Directory = Path.Combine(Path.GetTempPath(), "suru-emit-tests-" + Path.GetRandomFileName());
        System.IO.Directory.CreateDirectory(Directory);
    }

    /// <summary>The workspace root.</summary>
    public string Directory { get; }

    /// <summary>A source path inside the workspace. The file need not exist — the emit stage works
    /// from a layout, never by reading the source back. Accepts a nested relative path (e.g.
    /// <c>src/main.suru</c>), whose parent directories are created so the path is real on disk.</summary>
    public SourcePath SourcePath(string relativePath = "example.suru")
    {
        string fullPath = Path.Combine(Directory, relativePath);
        System.IO.Directory.CreateDirectory(Path.GetDirectoryName(fullPath)!);
        // Fully qualified: the enclosing namespace's `Compiler` class shadows the namespace name,
        // and this method's own name shadows the type.
        return global::Suru.Compiler.SourcePath.Create(fullPath);
    }

    /// <summary>
    /// Write an empty project marker, making <paramref name="relativeDirectory"/> (the workspace
    /// root by default) a project root. The marker's contents are never read, so it is left empty.
    /// </summary>
    public string MarkAsProject(string relativeDirectory = "")
    {
        string directory = Path.Combine(Directory, relativeDirectory);
        System.IO.Directory.CreateDirectory(directory);
        string markerPath = Path.Combine(directory, BuildLayout.ProjectMarkerFileName);
        File.WriteAllText(markerPath, string.Empty);
        return markerPath;
    }

    public void Dispose()
    {
        try
        {
            System.IO.Directory.Delete(Directory, recursive: true);
        }
        catch (DirectoryNotFoundException)
        {
            // Already gone — nothing to clean up.
        }
    }
}

/// <summary>Tests for <see cref="BuildLayout"/>, the artifact-path policy.</summary>
public class BuildLayoutTests
{
    /// <summary>
    /// With no project marker anywhere above it, a source file is its own project: artifacts go in a
    /// <c>build</c> folder beside it, named after the source stem — the object keeps a <c>.o</c>
    /// extension, the executable has none.
    /// </summary>
    [Fact]
    public void ForSource_StandaloneFile_PutsArtifactsInBuildFolderBesideSource()
    {
        using TempWorkspace workspace = new();

        BuildLayout layout = BuildLayout.ForSource(workspace.SourcePath("exit9.suru"));

        Assert.True(layout.IsStandaloneFile);
        Assert.Null(layout.ProjectMarkerPath);
        Assert.Equal(workspace.Directory, layout.ProjectRoot);
        Assert.Equal("exit9", layout.OutputName);
        Assert.Equal(Path.Combine(workspace.Directory, "build"), layout.BuildDirectory);
        Assert.Equal(Path.Combine(workspace.Directory, "build", "exit9.o"), layout.ObjectPath);
        Assert.Equal(Path.Combine(workspace.Directory, "build", "exit9"), layout.ExecutablePath);
    }

    /// <summary>
    /// The policy in one assertion: a project marker pulls the <c>build</c> folder up to the project
    /// root, however deep the source sits, and the artifacts take the root directory's name rather
    /// than the source stem. One project, one build folder, one output.
    /// </summary>
    [Fact]
    public void ForSource_UnderProjectMarker_PutsOneBuildFolderAtProjectRoot()
    {
        using TempWorkspace workspace = new();
        string markerPath = workspace.MarkAsProject();
        SourcePath sourcePath = workspace.SourcePath(Path.Combine("src", "deep", "main.suru"));
        string projectName = new DirectoryInfo(workspace.Directory).Name;

        BuildLayout layout = BuildLayout.ForSource(sourcePath);

        Assert.False(layout.IsStandaloneFile);
        Assert.Equal(markerPath, layout.ProjectMarkerPath);
        Assert.Equal(workspace.Directory, layout.ProjectRoot);
        Assert.Equal(projectName, layout.OutputName);
        Assert.Equal(Path.Combine(workspace.Directory, "build"), layout.BuildDirectory);
        Assert.Equal(Path.Combine(workspace.Directory, "build", projectName + ".o"), layout.ObjectPath);
        Assert.Equal(Path.Combine(workspace.Directory, "build", projectName), layout.ExecutablePath);
    }

    /// <summary>
    /// Two sources in different folders of the same project share one build directory and one
    /// output — the whole point of rooting the layout at the project rather than the file.
    /// </summary>
    [Fact]
    public void ForSource_TwoSourcesInOneProject_ShareOneBuildDirectoryAndOutput()
    {
        using TempWorkspace workspace = new();
        workspace.MarkAsProject();

        BuildLayout first = BuildLayout.ForSource(workspace.SourcePath(Path.Combine("src", "main.suru")));
        BuildLayout second = BuildLayout.ForSource(workspace.SourcePath(Path.Combine("lib", "util.suru")));

        Assert.Equal(first.BuildDirectory, second.BuildDirectory);
        Assert.Equal(first.ObjectPath, second.ObjectPath);
        Assert.Equal(first.ExecutablePath, second.ExecutablePath);
    }

    /// <summary>
    /// The nearest marker wins: a project nested inside another is rooted at itself, so the inner
    /// project's artifacts do not land in the outer project's build folder.
    /// </summary>
    [Fact]
    public void ForSource_NestedProject_RootsAtTheNearestMarker()
    {
        using TempWorkspace workspace = new();
        workspace.MarkAsProject();
        string innerMarkerPath = workspace.MarkAsProject("inner");
        string innerRoot = Path.Combine(workspace.Directory, "inner");

        BuildLayout layout = BuildLayout.ForSource(workspace.SourcePath(Path.Combine("inner", "main.suru")));

        Assert.Equal(innerMarkerPath, layout.ProjectMarkerPath);
        Assert.Equal(innerRoot, layout.ProjectRoot);
        Assert.Equal("inner", layout.OutputName);
        Assert.Equal(Path.Combine(innerRoot, "build"), layout.BuildDirectory);
    }

    /// <summary>
    /// A relative source path resolves to absolute artifact paths, so nothing downstream — least of
    /// all the linker's child process — depends on the working directory.
    /// </summary>
    [Fact]
    public void ForSource_RelativePath_ResolvesToAbsolutePaths()
    {
        string relativePath = Path.Combine("examples", "exit9.suru");

        BuildLayout layout = BuildLayout.ForSource(SourcePath.Create(relativePath));

        Assert.True(Path.IsPathRooted(layout.SourcePath.FullPath));
        Assert.True(Path.IsPathRooted(layout.ProjectRoot));
        Assert.True(Path.IsPathRooted(layout.ObjectPath));
        Assert.True(Path.IsPathRooted(layout.ExecutablePath));
        // Same layout as naming the file absolutely — the only difference is how it was spelled.
        BuildLayout absolute = BuildLayout.ForSource(
            SourcePath.Create(Path.Combine(Directory.GetCurrentDirectory(), relativePath)));
        Assert.Equal(absolute.ExecutablePath, layout.ExecutablePath);
    }

    /// <summary>
    /// Deriving a layout reads the filesystem (it probes for a project marker) but never writes to
    /// it — the build directory appears only when a caller asks for it.
    /// </summary>
    [Fact]
    public void ForSource_CreatesNothingOnDisk()
    {
        using TempWorkspace workspace = new();

        BuildLayout layout = BuildLayout.ForSource(workspace.SourcePath());

        Assert.False(Directory.Exists(layout.BuildDirectory));
    }

    /// <summary>
    /// A source path that resolves nowhere finds no marker and is treated as standalone, rather than
    /// throwing — callers compute a layout before deciding whether the build can proceed.
    /// </summary>
    [Fact]
    public void ForSource_NonexistentSource_IsStandalone()
    {
        using TempWorkspace workspace = new();
        SourcePath sourcePath = SourcePath.Create(
            Path.Combine(workspace.Directory, "no-such-folder", "ghost.suru"));

        BuildLayout layout = BuildLayout.ForSource(sourcePath);

        Assert.True(layout.IsStandaloneFile);
        Assert.Equal("ghost", layout.OutputName);
    }

    /// <summary>
    /// <see cref="BuildLayout.EnsureBuildDirectory"/> creates the folder and is idempotent —
    /// compiling twice must not fail on the second run.
    /// </summary>
    [Fact]
    public void EnsureBuildDirectory_CreatesFolderAndIsIdempotent()
    {
        using TempWorkspace workspace = new();
        BuildLayout layout = BuildLayout.ForSource(workspace.SourcePath());

        layout.EnsureBuildDirectory();
        layout.EnsureBuildDirectory();

        Assert.True(Directory.Exists(layout.BuildDirectory));
    }

    /// <summary>The argument guard — validation itself lives on <see cref="SourcePath"/>.</summary>
    [Fact]
    public void ForSource_NullSourcePath_Throws()
    {
        Assert.Throws<ArgumentNullException>(() => BuildLayout.ForSource(null!));
    }
}

/// <summary>Tests for <see cref="SourcePath"/> — the validated, absolute source-file path.</summary>
public class SourcePathTests
{
    /// <summary>
    /// A path with no file to build is a caller error, caught once here so nothing downstream has to
    /// re-check it. Asserted with <c>ThrowsAny</c> because a null argument correctly raises the
    /// <see cref="ArgumentNullException"/> subclass, and the contract is the base type.
    /// </summary>
    [Theory]
    [InlineData("")]
    [InlineData("   ")]
    [InlineData(null)]
    [InlineData("/")]
    public void Create_WithoutAFileName_Throws(string? path)
    {
        Assert.ThrowsAny<ArgumentException>(() => SourcePath.Create(path!));
    }

    /// <summary>A relative path resolves against the current directory and splits into its parts.</summary>
    [Fact]
    public void Create_RelativePath_ResolvesToAbsoluteAndSplitsParts()
    {
        SourcePath source = SourcePath.Create(Path.Combine("examples", "exit9.suru"));

        string expected = Path.Combine(Directory.GetCurrentDirectory(), "examples", "exit9.suru");
        Assert.Equal(expected, source.FullPath);
        Assert.Equal(Path.Combine(Directory.GetCurrentDirectory(), "examples"), source.Directory);
        Assert.Equal("exit9", source.Stem);
        Assert.Equal(expected, source.ToString());
    }

    /// <summary>Validation never touches the filesystem — a file that is not there is still a valid path.</summary>
    [Fact]
    public void Create_NonexistentFile_Succeeds()
    {
        SourcePath source = SourcePath.Create(Path.Combine(Path.GetTempPath(), "no-such-folder", "ghost.suru"));

        Assert.Equal("ghost", source.Stem);
        Assert.False(File.Exists(source.FullPath));
    }
}

/// <summary>Tests for <see cref="ObjectEmitter"/> — LLVM module to ELF object file.</summary>
public class ObjectEmitterTests
{
    /// <summary>The example program emits a real ELF object at the layout's object path.</summary>
    [Fact]
    public void EmitObject_ExampleProgram_WritesElfObjectFile()
    {
        using TempWorkspace workspace = new();
        BuildLayout layout = BuildLayout.ForSource(workspace.SourcePath());
        layout.EnsureBuildDirectory();
        using CodegenResult codegen = EmitTestHelpers.Generate(EmitTestHelpers.ExampleProgram);

        ObjectEmitter.EmitObject(codegen, layout.ObjectPath);

        Assert.True(File.Exists(layout.ObjectPath));
        Assert.True(EmitTestHelpers.IsElf(layout.ObjectPath));
        Assert.True(new FileInfo(layout.ObjectPath).Length > 0);
    }

    /// <summary>
    /// Emission stamps the module with the target triple and a data layout, so IR dumped after this
    /// point describes what it was compiled for rather than floating free of a target.
    /// </summary>
    [Fact]
    public void EmitObject_StampsTargetTripleAndDataLayoutOnModule()
    {
        using TempWorkspace workspace = new();
        BuildLayout layout = BuildLayout.ForSource(workspace.SourcePath());
        layout.EnsureBuildDirectory();
        using CodegenResult codegen = EmitTestHelpers.Generate(EmitTestHelpers.ExampleProgram);
        Assert.DoesNotContain("target triple", codegen.ToIrString());

        ObjectEmitter.EmitObject(codegen, layout.ObjectPath);

        string ir = codegen.ToIrString();
        Assert.Contains($"target triple = \"{ObjectEmitter.TargetTriple}\"", ir);
        Assert.Contains("target datalayout", ir);
    }

    /// <summary>Emitting twice overwrites, so a rebuild does not fail or append.</summary>
    [Fact]
    public void EmitObject_Twice_OverwritesTheObjectFile()
    {
        using TempWorkspace workspace = new();
        BuildLayout layout = BuildLayout.ForSource(workspace.SourcePath());
        layout.EnsureBuildDirectory();
        using CodegenResult codegen = EmitTestHelpers.Generate(EmitTestHelpers.ExampleProgram);

        ObjectEmitter.EmitObject(codegen, layout.ObjectPath);
        long firstLength = new FileInfo(layout.ObjectPath).Length;
        ObjectEmitter.EmitObject(codegen, layout.ObjectPath);

        Assert.Equal(firstLength, new FileInfo(layout.ObjectPath).Length);
    }

    /// <summary>
    /// A path in a directory that does not exist fails loudly: LLVM does not create parents, and
    /// silently succeeding-but-writing-nothing would surface as a confusing link error instead.
    /// </summary>
    [Fact]
    public void EmitObject_IntoMissingDirectory_Throws()
    {
        using TempWorkspace workspace = new();
        BuildLayout layout = BuildLayout.ForSource(workspace.SourcePath());
        using CodegenResult codegen = EmitTestHelpers.Generate(EmitTestHelpers.ExampleProgram);

        // Note: EnsureBuildDirectory deliberately not called.
        Assert.Throws<InvalidOperationException>(() => ObjectEmitter.EmitObject(codegen, layout.ObjectPath));
    }

    /// <summary>The argument guards — a caller error, not a compile failure.</summary>
    [Fact]
    public void EmitObject_NullCodegen_Throws()
    {
        Assert.Throws<ArgumentNullException>(() => ObjectEmitter.EmitObject(null!, "out.o"));
    }

    /// <summary>A blank output path names no file to write.</summary>
    [Theory]
    [InlineData("")]
    [InlineData("   ")]
    [InlineData(null)]
    public void EmitObject_WithBlankObjectPath_Throws(string? objectPath)
    {
        using CodegenResult codegen = EmitTestHelpers.Generate(EmitTestHelpers.ExampleProgram);

        Assert.ThrowsAny<ArgumentException>(() => ObjectEmitter.EmitObject(codegen, objectPath!));
    }
}

/// <summary>Tests for <see cref="Linker"/> — object file to runnable native ELF.</summary>
public class LinkerTests
{
    /// <summary>Emit an object for <paramref name="source"/> into a ready build directory.</summary>
    private static BuildLayout EmitObjectFor(TempWorkspace workspace, string source)
    {
        BuildLayout layout = BuildLayout.ForSource(workspace.SourcePath());
        layout.EnsureBuildDirectory();
        using CodegenResult codegen = EmitTestHelpers.Generate(source);
        ObjectEmitter.EmitObject(codegen, layout.ObjectPath);
        return layout;
    }

    /// <summary>
    /// Task 6's exit criterion, end to end: the object links into an executable ELF that actually
    /// runs — and, because Stage 1 observes results through the process exit code, running it also
    /// proves the whole chain computed <c>((1 + 2) * 3) = 9</c> rather than the math-precedence 7.
    /// </summary>
    [Fact]
    public void Link_EmittedObject_ProducesRunnableElf()
    {
        using TempWorkspace workspace = new();
        BuildLayout layout = EmitObjectFor(workspace, EmitTestHelpers.ExampleProgram);

        LinkResult result = Linker.Link(layout.ObjectPath, layout.ExecutablePath);

        Assert.True(result.Succeeded, result.Message);
        Assert.Equal(layout.ExecutablePath, result.ExecutablePath);
        Assert.True(File.Exists(layout.ExecutablePath));
        Assert.True(EmitTestHelpers.IsElf(layout.ExecutablePath));
        Assert.Equal(EmitTestHelpers.ExampleExitCode, EmitTestHelpers.RunExecutable(layout.ExecutablePath));
    }

    /// <summary>A clean link is silent — no driver output to show the user.</summary>
    [Fact]
    public void Link_Succeeding_ReportsNoOutput()
    {
        using TempWorkspace workspace = new();
        BuildLayout layout = EmitObjectFor(workspace, EmitTestHelpers.ExampleProgram);

        LinkResult result = Linker.Link(layout.ObjectPath, layout.ExecutablePath);

        Assert.True(result.Succeeded, result.Message);
        Assert.Equal(string.Empty, result.Message);
    }

    /// <summary>
    /// The reason linking returns a result instead of throwing: an <c>extern</c> naming a function
    /// that exists nowhere passes every earlier pass — the front end has no way to know what libc
    /// contains — and fails here. That is a user mistake, so it must be reported, not crash the
    /// compiler, and the message must carry the driver's explanation.
    /// </summary>
    [Fact]
    public void Link_ExternWithNoDefinition_ReportsFailureWithDriverOutput()
    {
        using TempWorkspace workspace = new();
        BuildLayout layout = EmitObjectFor(
            workspace,
            "extern fn suru_no_such_libc_function(i32)\n"
            + "\n"
            + "fn main() {\n"
            + "    suru_no_such_libc_function(1)\n"
            + "}\n");

        LinkResult result = Linker.Link(layout.ObjectPath, layout.ExecutablePath);

        Assert.False(result.Succeeded);
        Assert.Null(result.ExecutablePath);
        Assert.False(File.Exists(layout.ExecutablePath));
        Assert.Contains("suru_no_such_libc_function", result.Message);
    }

    /// <summary>An object file that is not there is reported, not thrown.</summary>
    [Fact]
    public void Link_MissingObjectFile_ReportsFailure()
    {
        using TempWorkspace workspace = new();
        BuildLayout layout = BuildLayout.ForSource(workspace.SourcePath());
        layout.EnsureBuildDirectory();

        LinkResult result = Linker.Link(layout.ObjectPath, layout.ExecutablePath);

        Assert.False(result.Succeeded);
        Assert.Null(result.ExecutablePath);
        Assert.NotEqual(string.Empty, result.Message);
    }

    /// <summary>The argument guards — blank paths are caller errors.</summary>
    [Theory]
    [InlineData("", "out")]
    [InlineData("   ", "out")]
    [InlineData(null, "out")]
    [InlineData("in.o", "")]
    [InlineData("in.o", "   ")]
    [InlineData("in.o", null)]
    public void Link_WithBlankPath_Throws(string? objectPath, string? executablePath)
    {
        Assert.ThrowsAny<ArgumentException>(() => Linker.Link(objectPath!, executablePath!));
    }
}
