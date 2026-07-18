namespace Suru.Compiler.Emit;

/// <summary>
/// Where a compilation's output artifacts go.
///
/// <para><b>One build folder per project, one compilation unit.</b> A build produces a single
/// object and a single executable for the whole project, both in one <c>build/</c> folder at the
/// project root — not one artifact per source file. Per-file objects buy incremental rebuilds and
/// link-time parallelism at the cost of cross-unit optimization and a scattered output tree; Suru
/// takes the other trade (whole-program compilation, one output directory), which also keeps
/// <c>rm -rf build</c> a complete clean.</para>
///
/// <para><b>Finding the root.</b> Walk up from the source file looking for a
/// <see cref="ProjectMarkerFileName"/> marker; the directory holding it is the project root. If no
/// marker exists anywhere up to the filesystem root, the source file <i>is</i> the project — a
/// standalone file, rooted at its own directory. That fallback is what makes a bare
/// <c>suru run hello.suru</c> work with no ceremony; the marker is what lets a real project put
/// sources in <c>src/</c> and still get one <c>build/</c> at the top.</para>
/// </summary>
public sealed class BuildLayout
{
    /// <summary>
    /// The file that marks a directory as a project root.
    /// </summary>
    public const string ProjectMarkerFileName = ".suru-project";

    /// <summary>Name of the output folder created at the project root.</summary>
    public const string BuildDirectoryName = "build";

    private BuildLayout(
        SourcePath sourcePath,
        string projectRoot,
        string? projectMarkerPath,
        string outputName,
        string buildDirectory,
        string objectPath,
        string executablePath)
    {
        SourcePath = sourcePath;
        ProjectRoot = projectRoot;
        ProjectMarkerPath = projectMarkerPath;
        OutputName = outputName;
        BuildDirectory = buildDirectory;
        ObjectPath = objectPath;
        ExecutablePath = executablePath;
    }

    /// <summary>The entry-point source file this layout was derived from.</summary>
    public SourcePath SourcePath { get; }

    /// <summary>
    /// The absolute path of the project root — the directory holding <see cref="ProjectMarkerPath"/>,
    /// or the source file's own directory when the source is a standalone file.
    /// </summary>
    public string ProjectRoot { get; }

    /// <summary>
    /// The absolute path of the <see cref="ProjectMarkerFileName"/> that established
    /// <see cref="ProjectRoot"/>, or null when no marker was found and the source stands alone.
    /// </summary>
    public string? ProjectMarkerPath { get; }

    /// <summary>True when no project marker was found, so the source file is its own project.</summary>
    public bool IsStandaloneFile => ProjectMarkerPath is null;

    /// <summary>
    /// The base name the artifacts are named after: the project root's directory name for a marked
    /// project, or the source file's stem for a standalone file. A standalone <c>hello.suru</c>
    /// building to <c>build/hello</c> reads better than naming it after whatever folder it sits in.
    /// </summary>
    public string OutputName { get; }

    /// <summary>The absolute path of the <c>build</c> folder at the project root.</summary>
    public string BuildDirectory { get; }

    /// <summary>The absolute path of the emitted object file (<c>build/&lt;name&gt;.o</c>).</summary>
    public string ObjectPath { get; }

    /// <summary>The absolute path of the linked executable (<c>build/&lt;name&gt;</c>, no extension —
    /// Linux gives an extension no meaning).</summary>
    public string ExecutablePath { get; }

    /// <summary>
    /// Derive the artifact layout for a compilation entered through <paramref name="sourcePath"/>.
    ///
    /// <para>This <b>reads the filesystem</b> — it probes each ancestor directory for a project
    /// marker — but it never writes: no directory is created here, so a caller can compute the
    /// layout before deciding whether to build. Neither the source file nor any ancestor is required
    /// to exist; a path that resolves nowhere simply finds no marker and is treated as standalone.</para>
    ///
    /// <para>The path arrives already validated and absolute — see <see cref="Suru.Compiler.SourcePath"/>,
    /// which is the only way to build one — so there is nothing left to check here.</para>
    /// </summary>
    /// <param name="sourcePath">The entry-point <c>.suru</c> file.</param>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="sourcePath"/> is null.</exception>
    public static BuildLayout ForSource(SourcePath sourcePath)
    {
        ArgumentNullException.ThrowIfNull(sourcePath);

        string? markerPath = FindProjectMarker(sourcePath.Directory);
        string projectRoot = markerPath is null ? sourcePath.Directory : Path.GetDirectoryName(markerPath)!;
        string outputName = markerPath is null ? sourcePath.Stem : DirectoryName(projectRoot) ?? sourcePath.Stem;

        string buildDirectory = Path.Combine(projectRoot, BuildDirectoryName);

        return new BuildLayout(
            sourcePath,
            projectRoot,
            markerPath,
            outputName,
            buildDirectory,
            Path.Combine(buildDirectory, outputName + ".o"),
            Path.Combine(buildDirectory, outputName));
    }

    /// <summary>
    /// Create the build directory if it does not exist. Neither LLVM's object emission nor
    /// <c>clang</c> creates missing parent directories, so this runs before either.
    /// </summary>
    /// <exception cref="IOException">Thrown when the directory cannot be created.</exception>
    public void EnsureBuildDirectory() => Directory.CreateDirectory(BuildDirectory);

    /// <summary>
    /// Walk from <paramref name="startDirectory"/> up to the filesystem root, returning the path of
    /// the first <see cref="ProjectMarkerFileName"/> found, or null if there is none. The nearest
    /// marker wins, so a project nested inside another is rooted at itself.
    /// </summary>
    private static string? FindProjectMarker(string startDirectory)
    {
        for (DirectoryInfo? directory = new(startDirectory); directory is not null; directory = directory.Parent)
        {
            string candidate = Path.Combine(directory.FullName, ProjectMarkerFileName);
            if (File.Exists(candidate))
            {
                return candidate;
            }
        }

        return null;
    }

    /// <summary>
    /// The final segment of a directory path, or null when there is none — a marker sitting at the
    /// filesystem root leaves nothing to name the output after, and the caller falls back to the
    /// source stem.
    /// </summary>
    private static string? DirectoryName(string directory)
    {
        string name = new DirectoryInfo(directory).Name;
        return name.Length == 0 || name == Path.DirectorySeparatorChar.ToString() ? null : name;
    }
}
