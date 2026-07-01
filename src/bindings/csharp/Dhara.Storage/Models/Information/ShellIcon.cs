namespace Dhara.Storage.Models.Information;

/// <summary>
/// OS shell icon pixels in uncompressed row-major RGBA layout (not PNG).
/// </summary>
public sealed record ShellIcon(int Width, int Height, ReadOnlyMemory<byte> RgbaPixels)
{
    /// <summary>Expected byte length (<c>Width * Height * 4</c>).</summary>
    public int ByteLength => Width * Height * 4;

    /// <summary>Whether the pixel buffer length matches the declared dimensions.</summary>
    public bool IsValid => RgbaPixels.Length == ByteLength;
}
