using System.Collections.Generic;
using Newtonsoft.Json.Linq;
using UnityEngine;

/// <summary>
/// Composes a map's terrain into one texture from its tile rows and the legend
/// (character → [terrain id, walkable]); tiles are Resources/Sprites/tile_&lt;id&gt;.png (16×16).
/// </summary>
public static class TileMap
{
    const int S = 16;
    static readonly Dictionary<string, Color32[]> Tiles = new();

    static Color32[] Tile(string id)
    {
        if (Tiles.TryGetValue(id, out var px)) return px;
        var tex = Resources.Load<Texture2D>("Sprites/tile_" + id);
        px = tex != null && tex.isReadable ? tex.GetPixels32() : null;
        Tiles[id] = px;
        return px;
    }

    public static Texture2D Build(JToken layer, JObject legend)
    {
        if (legend == null || layer?["tiles"] is not JArray rows || rows.Count == 0) return null;
        int w = (int)layer["width"], h = (int)layer["height"];
        var tex = new Texture2D(w * S, h * S, TextureFormat.RGBA32, false) { filterMode = FilterMode.Point, wrapMode = TextureWrapMode.Clamp };
        var all = new Color32[w * S * h * S];
        var fallback = new Color32(120, 200, 80, 255);
        for (int y = 0; y < h; y++)
        {
            string row = y < rows.Count ? (string)rows[y] : "";
            for (int x = 0; x < w; x++)
            {
                string ch = x < row.Length ? row[x].ToString() : ".";
                string id = (string)legend[ch]?[0];
                var px = id != null ? Tile(id) : null;
                // Texture rows start at the bottom.
                int oy = (h - 1 - y) * S, ox = x * S;
                for (int ty = 0; ty < S; ty++)
                for (int tx = 0; tx < S; tx++)
                    all[(oy + ty) * w * S + ox + tx] = px != null ? px[ty * S + tx] : fallback;
            }
        }
        tex.SetPixels32(all);
        tex.Apply();
        return tex;
    }
}
