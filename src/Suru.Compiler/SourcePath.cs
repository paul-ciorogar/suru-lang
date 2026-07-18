namespace Suru.Compiler;

/// <summary>
/// A validated path to a Suru source file, resolved to an absolute path.
/// </summary>
public sealed record SourcePath
{
    private SourcePath(string fullPath, string directory, string stem)
    {
        FullPath = fullPath;
        Directory = directory;
        Stem = stem;
    }

    /// <summary>The absolute path of the source file.</summary>
    public string FullPath { get; }

    /// <summary>The absolute path of the directory holding the source file.</summary>
    public string Directory { get; }

    /// <summary>The file name without its extension — what a standalone build is named after.</summary>
    public string Stem { get; }

    /// <summary>
    /// Validate <paramref name="path"/> and resolve it to an absolute <see cref="SourcePath"/>.
    /// Relative paths resolve against the current directory, so nothing downstream depends on the
    /// working directory.
    /// </summary>
    /// <exception cref="ArgumentException">
    /// Thrown when <paramref name="path"/> is null, empty, whitespace, or names no file (e.g. a bare
    /// directory separator) — there would be no source to enter the compilation through.
    /// </exception>
    public static SourcePath Create(string path)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(path);

        string fullPath = Path.GetFullPath(path);
        string stem = Path.GetFileNameWithoutExtension(fullPath);
        if (string.IsNullOrEmpty(stem))
        {
            throw new ArgumentException($"Source path '{path}' does not name a file.", nameof(path));
        }

        // GetFullPath always yields a rooted path, so the directory is never null here.
        return new SourcePath(fullPath, Path.GetDirectoryName(fullPath)!, stem);
    }

    public override string ToString() => FullPath;
}
