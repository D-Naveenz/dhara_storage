namespace Dhara.Storage.Models.Metadata;

/// <summary>
/// OS shell icon pixels in uncompressed row-major RGBA layout (not PNG).
/// </summary>
/// <param name="Width">Icon width in pixels.</param>
/// <param name="Height">Icon height in pixels.</param>
/// <param name="RgbaPixels">Raw RGBA bytes; expected length is <c>Width * Height * 4</c>.</param>
public sealed record ShellIcon(int Width, int Height, ReadOnlyMemory<byte> RgbaPixels)
{
    /// <summary>Expected byte length (<c>Width * Height * 4</c>).</summary>
    public int ByteLength => Width * Height * 4;

    /// <summary>Whether the pixel buffer length matches the declared dimensions.</summary>
    public bool IsValid => RgbaPixels.Length == ByteLength;
}
