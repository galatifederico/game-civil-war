using System.Collections.Generic;
using UnityEngine;

// La panoramica delle aree (board) del mondo: dove sta ogni area rispetto alle altre e una
// miniatura del suo terreno. Le board non hanno coordinate globali: la posizione si ricava dai
// passaggi. Un passaggio vicino al bordo destro di A che porta in B mette B a destra di A, e cosi'
// via per gli altri lati; le aree che nessun passaggio raggiunge vanno in fondo.
public static class AreaMap
{
    static readonly Dictionary<string, Texture2D> thumbnails = new Dictionary<string, Texture2D>();

    public static void ClearCache()
    {
        foreach (var t in thumbnails.Values) Object.Destroy(t);
        thumbnails.Clear();
    }

    // Colonna e riga di ogni area (con 0,0 in alto a sinistra, senza buchi ai bordi).
    public static Dictionary<string, Vector2Int> Layout(List<BoardData> boards)
    {
        var pos = new Dictionary<string, Vector2Int>();
        if (boards.Count == 0) return pos;
        var byId = new Dictionary<string, BoardData>();
        foreach (var b in boards) byId[b.id] = b;
        var taken = new HashSet<Vector2Int>();

        foreach (var start in boards)
        {
            if (pos.ContainsKey(start.id)) continue;
            // Un'area non raggiunta da nessun passaggio parte in una riga nuova sotto le altre.
            int row = 0;
            foreach (var p in pos.Values) row = Mathf.Max(row, p.y + 1);
            var origin = pos.Count == 0 ? Vector2Int.zero : new Vector2Int(0, row);
            while (taken.Contains(origin)) origin.x++;
            pos[start.id] = origin;
            taken.Add(origin);

            var queue = new Queue<BoardData>();
            queue.Enqueue(start);
            while (queue.Count > 0)
            {
                var b = queue.Dequeue();
                foreach (var g in b.gateways ?? new GatewayData[0])
                {
                    if (pos.ContainsKey(g.to_board) || !byId.TryGetValue(g.to_board, out var target)) continue;
                    var step = EdgeDirection(b, g.x, g.y);
                    var cell = pos[b.id] + step;
                    while (taken.Contains(cell)) cell += step;
                    pos[target.id] = cell;
                    taken.Add(cell);
                    queue.Enqueue(target);
                }
            }
        }

        int minX = int.MaxValue, minY = int.MaxValue;
        foreach (var p in pos.Values) { minX = Mathf.Min(minX, p.x); minY = Mathf.Min(minY, p.y); }
        var result = new Dictionary<string, Vector2Int>();
        foreach (var kv in pos) result[kv.Key] = kv.Value - new Vector2Int(minX, minY);
        return result;
    }

    // Da che lato di una board sta una casella: il lato piu' vicino.
    static Vector2Int EdgeDirection(BoardData b, int x, int y)
    {
        int left = x, right = b.width - 1 - x, up = y, down = b.height - 1 - y;
        int best = Mathf.Min(Mathf.Min(left, right), Mathf.Min(up, down));
        if (best == right) return new Vector2Int(1, 0);
        if (best == left) return new Vector2Int(-1, 0);
        if (best == down) return new Vector2Int(0, 1);
        return new Vector2Int(0, -1);
    }

    // Una miniatura di un pixel per casella (il filtro Point la tiene nitida): il terreno, i
    // passaggi in viola.
    public static Texture2D Thumbnail(BoardData b)
    {
        string key = b.id + ":" + b.width + "x" + b.height + ":" + (b.terrain != null ? string.Join("", b.terrain).GetHashCode() : 0);
        if (thumbnails.TryGetValue(key, out var cached)) return cached;
        var tex = new Texture2D(b.width, b.height, TextureFormat.RGBA32, false) { filterMode = FilterMode.Point, wrapMode = TextureWrapMode.Clamp, hideFlags = HideFlags.HideAndDontSave };
        var gateways = new HashSet<Vector2Int>();
        foreach (var g in b.gateways ?? new GatewayData[0]) gateways.Add(new Vector2Int(g.x, g.y));
        var pixels = new Color32[b.width * b.height];
        for (int y = 0; y < b.height; y++)
        {
            for (int x = 0; x < b.width; x++)
            {
                Color32 c = gateways.Contains(new Vector2Int(x, y)) ? new Color32(192, 132, 252, 255) : GlyphColor(TerrainArt.At(b.terrain, x, y));
                pixels[(b.height - 1 - y) * b.width + x] = c;
            }
        }
        tex.SetPixels32(pixels);
        tex.Apply();
        return thumbnails[key] = tex;
    }

    static Color32 GlyphColor(char glyph)
    {
        switch (glyph)
        {
            case TerrainArt.Flowers: return new Color32(232, 212, 77, 255);
            case TerrainArt.Path: return new Color32(201, 143, 90, 255);
            case TerrainArt.Stone: return new Color32(169, 176, 184, 255);
            case TerrainArt.Tree: return new Color32(46, 125, 50, 255);
            case TerrainArt.Bush: return new Color32(90, 168, 90, 255);
            case TerrainArt.Fence: return new Color32(139, 90, 43, 255);
            case TerrainArt.Wall: return new Color32(91, 98, 112, 255);
            default: return new Color32(123, 201, 111, 255);
        }
    }
}
