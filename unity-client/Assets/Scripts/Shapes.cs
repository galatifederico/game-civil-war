using System.Collections.Generic;
using UnityEngine;

/// <summary>
/// Very simple procedural sprites (circle, square, triangle, diamond, cross) generated once and tinted
/// with the colours of the data-driven sprite mapping. Real sprite sheets can replace them later.
/// </summary>
public static class Shapes
{
    const int Size = 64;
    static readonly Dictionary<string, Sprite> Cache = new();

    public static Sprite Get(string shape)
    {
        shape = string.IsNullOrEmpty(shape) ? "circle" : shape;
        if (Cache.TryGetValue(shape, out var s)) return s;
        var tex = new Texture2D(Size, Size, TextureFormat.RGBA32, false) { filterMode = FilterMode.Bilinear, wrapMode = TextureWrapMode.Clamp };
        var px = new Color32[Size * Size];
        for (int y = 0; y < Size; y++)
        for (int x = 0; x < Size; x++)
        {
            float u = (x + 0.5f) / Size * 2f - 1f, v = (y + 0.5f) / Size * 2f - 1f;
            bool inside = shape switch
            {
                "square" => Mathf.Abs(u) <= 0.9f && Mathf.Abs(v) <= 0.9f,
                "triangle" => v >= -0.85f && Mathf.Abs(u) <= (0.85f - v) * 0.55f,
                "diamond" => Mathf.Abs(u) + Mathf.Abs(v) <= 0.95f,
                "cross" => (Mathf.Abs(u - v) < 0.22f || Mathf.Abs(u + v) < 0.22f) && Mathf.Abs(u) < 0.85f && Mathf.Abs(v) < 0.85f,
                "ring" => u * u + v * v <= 0.95f && u * u + v * v >= 0.6f,
                _ => u * u + v * v <= 0.9f,
            };
            px[y * Size + x] = inside ? new Color32(255, 255, 255, 255) : new Color32(255, 255, 255, 0);
        }
        tex.SetPixels32(px);
        tex.Apply();
        s = Sprite.Create(tex, new Rect(0, 0, Size, Size), new Vector2(0.5f, 0.5f), Size);
        s.name = shape;
        Cache[shape] = s;
        return s;
    }

    /// <summary>1x1 white sprite (pixels per unit 1) for zones, bars and backgrounds.</summary>
    public static Sprite Pixel => Get1x1();

    static Sprite _pixel;

    static Sprite Get1x1()
    {
        if (_pixel != null) return _pixel;
        var tex = new Texture2D(1, 1) { filterMode = FilterMode.Point };
        tex.SetPixel(0, 0, Color.white);
        tex.Apply();
        _pixel = Sprite.Create(tex, new Rect(0, 0, 1, 1), new Vector2(0f, 1f), 1);
        return _pixel;
    }

    public static Color Parse(string hex, Color fallback)
    {
        return !string.IsNullOrEmpty(hex) && ColorUtility.TryParseHtmlString(hex, out var c) ? c : fallback;
    }

    /// <summary>Stable pastel colour for ids without a sprite mapping.</summary>
    public static Color FromId(string id)
    {
        if (string.IsNullOrEmpty(id)) return new Color(0.6f, 0.6f, 0.6f);
        uint h = 2166136261;
        foreach (var ch in id) h = (h ^ ch) * 16777619;
        return Color.HSVToRGB((h % 360) / 360f, 0.45f, 0.85f);
    }
}
