using System.Collections.Generic;
using Newtonsoft.Json.Linq;
using UnityEngine;

/// <summary>
/// Composes a map's terrain into one texture from its tile rows and the legend
/// (character → [terrain id, walkable]); tiles are Resources/Sprites/tile_&lt;id&gt;.png (16×16).
/// Pokémon-like touches: rock walls show their front face above walkable ground, water gets a lighter
/// shore line, solid tiles cast a short shadow, trees are drawn as tall sprites by <see cref="Trees"/>.
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

    /// Terrain drawn as a separate tall sprite: the cell itself shows this ground tile instead.
    public static readonly Dictionary<string, string> TallTiles = new() { ["albero"] = "erba" };

    static string Id(JArray rows, JObject legend, int x, int y)
    {
        if (y < 0 || y >= rows.Count) return null;
        string row = (string)rows[y];
        if (x < 0 || x >= row.Length) return null;
        return (string)legend[row[x].ToString()]?[0];
    }

    static bool Walkable(JArray rows, JObject legend, int x, int y)
    {
        if (y < 0 || y >= rows.Count) return false;
        string row = (string)rows[y];
        if (x < 0 || x >= row.Length) return false;
        var l = legend[row[x].ToString()];
        return l != null && (bool)l[1];
    }

    static bool IsWater(string id) => id == "acqua" || id == "piscina";

    public static Texture2D Build(JToken layer, JObject legend)
    {
        if (legend == null || layer?["tiles"] is not JArray rows || rows.Count == 0) return null;
        int w = (int)layer["width"], h = (int)layer["height"];
        int W = w * S;
        var tex = new Texture2D(W, h * S, TextureFormat.RGBA32, false) { filterMode = FilterMode.Point, wrapMode = TextureWrapMode.Clamp };
        var all = new Color32[W * h * S];
        var fallback = new Color32(120, 200, 80, 255);
        var shore = new Color32(200, 235, 255, 255);
        for (int y = 0; y < h; y++)
        for (int x = 0; x < w; x++)
        {
            string id = Id(rows, legend, x, y);
            if (id != null && TallTiles.TryGetValue(id, out var under)) id = under;
            // Cliff/rock front: a solid tile above walkable ground shows its face.
            if (id != null && !Walkable(rows, legend, x, y) && Walkable(rows, legend, x, y + 1) && Tile(id + "_face") != null)
                id += "_face";
            var px = id != null ? Tile(id) : null;
            int oy = (h - 1 - y) * S, ox = x * S; // texture rows start at the bottom
            for (int ty = 0; ty < S; ty++)
            for (int tx = 0; tx < S; tx++)
                all[(oy + ty) * W + ox + tx] = px != null ? px[ty * S + tx] : fallback;
            if (id == null) continue;
            if (IsWater(id))
            {
                // Shore: light line where water touches land.
                bool n = !IsWater(Id(rows, legend, x, y - 1) ?? "acqua"), s = !IsWater(Id(rows, legend, x, y + 1) ?? "acqua");
                bool wv = !IsWater(Id(rows, legend, x - 1, y) ?? "acqua"), e = !IsWater(Id(rows, legend, x + 1, y) ?? "acqua");
                for (int i = 0; i < S; i++)
                {
                    if (n) all[(oy + S - 1) * W + ox + i] = shore;
                    if (s) all[oy * W + ox + i] = shore;
                    if (wv) all[(oy + i) * W + ox] = shore;
                    if (e) all[(oy + i) * W + ox + S - 1] = shore;
                }
            }
            else if (Walkable(rows, legend, x, y) && y > 0 && !Walkable(rows, legend, x, y - 1) && !IsWater(Id(rows, legend, x, y - 1) ?? "acqua"))
            {
                // Soft shadow at the foot of walls, rocks and furniture.
                for (int ty = S - 3; ty < S; ty++)
                for (int tx = 0; tx < S; tx++)
                {
                    ref var c = ref all[(oy + ty) * W + ox + tx];
                    float k = ty == S - 1 ? 0.7f : ty == S - 2 ? 0.8f : 0.9f;
                    c = new Color32((byte)(c.r * k), (byte)(c.g * k), (byte)(c.b * k), c.a);
                }
            }
        }
        tex.SetPixels32(all);
        tex.Apply();
        return tex;
    }

    /// Cells drawn as tall sprites (trees), as (x, y) map coordinates.
    public static IEnumerable<Vector2Int> Trees(JToken layer, JObject legend)
    {
        if (legend == null || layer?["tiles"] is not JArray rows) yield break;
        for (int y = 0; y < rows.Count; y++)
        {
            string row = (string)rows[y];
            for (int x = 0; x < row.Length; x++)
                if ((string)legend[row[x].ToString()]?[0] == "albero") yield return new Vector2Int(x, y);
        }
    }
}
