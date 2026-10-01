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
        var tex = new Texture2D(w * S, h * S, TextureFormat.RGBA32, false) { filterMode = FilterMode.Point, wrapMode = TextureWrapMode.Clamp };
        var all = new Color32[w * S * h * S];
        for (int y = 0; y < h; y++)
        for (int x = 0; x < w; x++)
            DrawCell(all, w, h, rows, legend, x, y);
        tex.SetPixels32(all);
        tex.Apply();
        return tex;
    }

    /// Redraws the cells around changed ones (their neighbours' faces, shores and shadows depend on them).
    public static void Patch(Texture2D tex, JToken layer, JObject legend, IEnumerable<Vector2Int> changed)
    {
        if (tex == null || legend == null || layer?["tiles"] is not JArray rows) return;
        int w = (int)layer["width"], h = (int)layer["height"];
        var cells = new HashSet<Vector2Int>();
        foreach (var c in changed)
            for (int dy = -1; dy <= 1; dy++)
            for (int dx = -1; dx <= 1; dx++)
                if (c.x + dx >= 0 && c.x + dx < w && c.y + dy >= 0 && c.y + dy < h) cells.Add(new Vector2Int(c.x + dx, c.y + dy));
        var block = new Color32[S * S];
        foreach (var c in cells)
        {
            DrawCell(block, 1, 1, rows, legend, c.x, c.y, c.y);
            tex.SetPixels32(c.x * S, (h - 1 - c.y) * S, S, S, block);
        }
        tex.Apply();
    }

    /// Draws one cell into `all` (a w×h-cell buffer). `row0` is the buffer row of map row `y`.
    static void DrawCell(Color32[] all, int w, int h, JArray rows, JObject legend, int x, int y, int row0 = -1)
    {
        var fallback = new Color32(120, 200, 80, 255);
        var shore = new Color32(200, 235, 255, 255);
        int W = w * S;
        int oy = row0 >= 0 ? 0 : (h - 1 - y) * S, ox = row0 >= 0 ? 0 : x * S; // texture rows start at the bottom
        string id = Id(rows, legend, x, y);
        if (id != null && TallTiles.TryGetValue(id, out var under)) id = under;
        // Cliff/rock front: a solid tile above walkable ground shows its face.
        if (id != null && !Walkable(rows, legend, x, y) && Walkable(rows, legend, x, y + 1) && Tile(id + "_face") != null)
            id += "_face";
        var px = id != null ? Tile(id) : null;
        for (int ty = 0; ty < S; ty++)
        for (int tx = 0; tx < S; tx++)
            all[(oy + ty) * W + ox + tx] = px != null ? px[ty * S + tx] : fallback;
        if (id == null) return;
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

    /// Replaces one character of the layer's tile rows (a dug cell, an admin edit).
    public static void SetChar(JToken layer, int x, int y, string ch)
    {
        if (layer?["tiles"] is not JArray rows || y < 0 || y >= rows.Count) return;
        var row = (string)rows[y];
        if (x < 0 || x >= row.Length || string.IsNullOrEmpty(ch)) return;
        rows[y] = row.Substring(0, x) + ch[0] + row.Substring(x + 1);
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
