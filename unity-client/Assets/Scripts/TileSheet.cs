using System;
using System.Collections.Generic;
using UnityEngine;

// I tile del terreno: la tavola "Tiny Town" di Kenney (CC0, 16x16 px, 12 colonne x 11 righe, senza
// spazio tra i tile), letta a runtime da Resources/Art/kenney-tiny-town.bytes (un PNG rinominato:
// cosi' non c'e' compressione e le impostazioni di importazione non contano). Licenza accanto al file.
public static class TileSheet
{
    public const int TileSize = 16;
    const int Columns = 12, Rows = 11;

    static Texture2D texture;
    static bool triedToLoad;
    static readonly Dictionary<string, Sprite> sprites = new Dictionary<string, Sprite>();

    // Indici dei tile nella tavola (numerati per riga, da in alto a sinistra).
    public const int Grass = 0, GrassTufts = 1, GrassFlowers = 2;
    public const int TreeTop = 4, TreeBase = 16, AutumnTreeTop = 3, AutumnTreeBase = 15;
    public const int Bush = 5;
    public const int Stone = 43;
    public const int Wall = 126;
    public const int FenceLeftEnd = 80, FenceMiddle = 81, FenceRightEnd = 82, FenceVertical = 59;
    // Sentiero di terra con il bordo d'erba, in 3x3: angoli, lati e centro.
    public static readonly int[,] Path = { { 12, 13, 14 }, { 24, 25, 26 }, { 36, 37, 38 } };

    static Texture2D Texture
    {
        get
        {
            if (texture != null || triedToLoad) return texture;
            triedToLoad = true;
            var asset = Resources.Load<TextAsset>("Art/kenney-tiny-town");
            if (asset == null)
            {
                Debug.LogWarning("TileSheet: manca Resources/Art/kenney-tiny-town.bytes, uso tile a tinta unita.");
                return null;
            }
            texture = new Texture2D(2, 2, TextureFormat.RGBA32, false) { filterMode = FilterMode.Point, wrapMode = TextureWrapMode.Clamp, hideFlags = HideFlags.HideAndDontSave };
            texture.LoadImage(asset.bytes);
            return texture;
        }
    }

    // Uno sprite della tavola, con il perno al centro (o in basso, per le cose alte come gli alberi).
    public static Sprite Tile(int index, bool pivotAtBottom = false)
    {
        string key = index + (pivotAtBottom ? "b" : "c");
        if (sprites.TryGetValue(key, out var sprite)) return sprite;
        var tex = Texture;
        if (tex == null) return sprites[key] = Solid(index);
        int col = index % Columns, row = index / Columns;
        var rect = new Rect(col * TileSize, tex.height - (row + 1) * TileSize, TileSize, TileSize);
        return sprites[key] = Sprite.Create(tex, rect, new Vector2(0.5f, pivotAtBottom ? 0f : 0.5f), PixelArt.PixelsPerUnit);
    }

    // Senza la tavola il gioco resta giocabile: ogni tile diventa un quadrato colorato.
    static Sprite Solid(int index)
    {
        var color = index <= 2 ? new Color(0.48f, 0.79f, 0.44f) : index >= 12 && index <= 42 ? new Color(0.79f, 0.56f, 0.35f) : new Color(0.3f, 0.5f, 0.3f);
        var tex = new Texture2D(1, 1) { filterMode = FilterMode.Point, hideFlags = HideFlags.HideAndDontSave };
        tex.SetPixel(0, 0, color);
        tex.Apply();
        return Sprite.Create(tex, new Rect(0, 0, 1, 1), new Vector2(0.5f, 0.5f), 1f);
    }
}

// Come si disegna il terreno che manda il server (una stringa per riga, un carattere per casella,
// vedi internal/game/terrain.go). L'erba varia in modo deterministico (ciuffi e fiori) e i
// sentieri, le staccionate e gli alberi scelgono il tile giusto guardando i vicini.
public static class TerrainArt
{
    public const char Grass = '.', Flowers = ',', Path = '=', Stone = ':', Tree = 'T', Bush = 'B', Fence = 'F', Wall = '#';

    public static char At(string[] rows, int x, int y)
    {
        if (rows == null || y < 0 || y >= rows.Length || x < 0 || x >= rows[y].Length) return Grass;
        return rows[y][x];
    }

    // Un numero fisso per casella, senza stato: la stessa casella e' sempre uguale.
    public static int Hash(int x, int y, int salt = 0)
    {
        unchecked
        {
            uint h = (uint)(x * 73856093 ^ y * 19349663 ^ salt * 83492791);
            h ^= h >> 13;
            h *= 0x5bd1e995;
            h ^= h >> 15;
            return (int)(h & 0x7fffffff);
        }
    }

    public static int GrassTile(int x, int y)
    {
        int h = Hash(x, y) % 100;
        return h < 8 ? TileSheet.GrassTufts : h < 11 ? TileSheet.GrassFlowers : TileSheet.Grass;
    }

    // Il tile del suolo sotto la casella (per gli alberi, i cespugli, le staccionate e i muri e' erba).
    public static int Ground(string[] rows, int x, int y)
    {
        switch (At(rows, x, y))
        {
            case Flowers: return TileSheet.GrassFlowers;
            case Stone: return TileSheet.Stone;
            case Path:
                bool n = At(rows, x, y - 1) == Path, s = At(rows, x, y + 1) == Path, w = At(rows, x - 1, y) == Path, e = At(rows, x + 1, y) == Path;
                // Con un vicino solo da un lato il bordo d'erba starebbe da entrambe: si usa il centro.
                int col = !w && !e ? 1 : !w ? 0 : !e ? 2 : 1;
                int row = !n && !s ? 1 : !n ? 0 : !s ? 2 : 1;
                return TileSheet.Path[row, col];
            default: return GrassTile(x, y);
        }
    }

    // Cio' che sta sopra il suolo: alberi (due tile, la cima sporge sulla casella sopra), cespugli,
    // staccionate, muri. Ritorna il tile della base e, per gli alberi, quello della cima (o -1).
    public static bool Decoration(string[] rows, int x, int y, out int baseTile, out int topTile, out bool pivotAtBottom)
    {
        baseTile = topTile = -1;
        pivotAtBottom = false;
        switch (At(rows, x, y))
        {
            case Tree:
                bool autumn = Hash(x, y, 7) % 100 < 22;
                baseTile = autumn ? TileSheet.AutumnTreeBase : TileSheet.TreeBase;
                topTile = autumn ? TileSheet.AutumnTreeTop : TileSheet.TreeTop;
                return true;
            case Bush:
                baseTile = TileSheet.Bush;
                return true;
            case Wall:
                baseTile = TileSheet.Wall;
                return true;
            case Fence:
                bool w = At(rows, x - 1, y) == Fence, e = At(rows, x + 1, y) == Fence;
                bool v = At(rows, x, y - 1) == Fence || At(rows, x, y + 1) == Fence;
                baseTile = v && !w && !e ? TileSheet.FenceVertical
                    : e && !w ? TileSheet.FenceLeftEnd
                    : w && !e ? TileSheet.FenceRightEnd
                    : TileSheet.FenceMiddle;
                return true;
        }
        return false;
    }
}
