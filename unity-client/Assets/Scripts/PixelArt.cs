using System;
using System.Collections.Generic;
using UnityEngine;

// Grafica 2D isometrica in pixel art, disegnata in codice: nessun asset esterno da importare.
// Gli sprite sono in scala di grigi (bianco = colore pieno) e si colorano con SpriteRenderer.color,
// cosi' lo stesso disegno serve a tutte le squadre. Le texture usano il filtro Point (niente sfocatura).
public static class PixelArt
{
    public const int PixelsPerUnit = 32;

    static Sprite squareTile, hexTile, shadow, ring, portal;
    static readonly Dictionary<string, Sprite> pieces = new Dictionary<string, Sprite>();

    // Blocco di terreno visto in isometrica: 32 px di larghezza = 1 unita' = una casella.
    public static Sprite Tile(string grid)
    {
        if (grid == GridMath.Hex) return hexTile ??= BuildSlab(32, 19, 4, HexHalfHeight);
        return squareTile ??= BuildSlab(32, 16, 4, dx => Mathf.Abs(dx) <= 16f ? 8f * (1f - Mathf.Abs(dx) / 16f) : -1f);
    }

    public static Sprite Shadow => shadow ??= BuildEllipse(12, 5, false, new Color(0f, 0f, 0f, 0.35f));
    public static Sprite Ring => ring ??= BuildEllipse(18, 8, true, new Color(1f, 0.92f, 0.25f, 1f));
    public static Sprite Portal => portal ??= BuildEllipse(20, 9, true, new Color(0.85f, 0.55f, 1f, 1f));

    public static Sprite Piece(string kind)
    {
        if (!pieces.TryGetValue(kind, out var sprite))
        {
            string[] rows;
            switch (kind)
            {
                case Kinds.Champion: rows = ChampionArt; break;
                case Kinds.Minor: rows = MinorArt; break;
                case Kinds.Npc: rows = NpcArt; break;
                case Kinds.Structure: rows = StructureArt; break;
                default: rows = ItemArt; break;
            }
            sprite = pieces[kind] = FromRows(rows);
        }
        return sprite;
    }

    // Icone degli oggetti (inventario): a colori veri, non tinte per squadra. Il server dice quale
    // usare (game.ItemIcons); una chiave sconosciuta ripiega sulla scatola.
    static readonly Dictionary<string, Texture2D> itemIcons = new Dictionary<string, Texture2D>();

    public static Texture2D ItemIcon(string key)
    {
        if (string.IsNullOrEmpty(key) || !IconArt.ContainsKey(key)) key = "box";
        if (!itemIcons.TryGetValue(key, out var tex))
            tex = itemIcons[key] = FromColorRows(IconArt[key]);
        return tex;
    }

    // Legenda: X contorno, r/R rosso, y/Y oro, b/B/c blu, n/N legno, s/S acciaio, w bianco, p rosa.
    static readonly Dictionary<string, string[]> IconArt = new Dictionary<string, string[]>
    {
        ["potion"] = new[]
        {
            "....XXXX....",
            "....XssX....",
            ".....XX.....",
            "....XssX....",
            "...XwwwwX...",
            "..XwrrrrwX..",
            "..XrrprrrX..",
            "..XrpprrrX..",
            "..XrrrrrrX..",
            "..XRrrrrRX..",
            "...XRRRRX...",
            "....XXXX....",
        },
        ["coin"] = new[]
        {
            "...XXXXXX...",
            "..XyyyyyyX..",
            ".XywyyyyyyX.",
            "XyyyYYYYyyyX",
            "XyyYyyyyYyyX",
            "XyyYyyyyYyyX",
            "XyyYyyyyYyyX",
            "XyyyYYYYyyyX",
            ".XyyyyyyyyX.",
            "..XyyyyyyX..",
            "...XXXXXX...",
        },
        ["sword"] = new[]
        {
            "........XXX.",
            ".......XwsX.",
            "......XwsX..",
            ".....XwsX...",
            "..X.XwsX....",
            "..XXwsX.....",
            "...XwX......",
            "..XyyyX.....",
            ".XyXXyyX....",
            "XnXX.XyX....",
            "XNX...X.....",
            ".X..........",
        },
        ["gem"] = new[]
        {
            "...XXXXXX...",
            "..XcwwcbbX..",
            ".XcwcbbbbBX.",
            "XcccbbbbbBBX",
            ".XcbbbbbbBX.",
            "..XcbbbbBX..",
            "...XcbbBX...",
            "....XcBX....",
            ".....XX.....",
        },
        ["chest"] = new[]
        {
            "..XXXXXXXX..",
            ".XnnnnnnnnX.",
            "XnNNNNNNNNnX",
            "XXXXXyyXXXXX",
            "XnnnnyyynnnX",
            "XnNNNNNNNNnX",
            "XnnnnnnnnnnX",
            "XNNNNNNNNNNX",
            ".XXXXXXXXXX.",
        },
        ["sign"] = new[]
        {
            ".XXXXXXXXXX.",
            "XnnnnnnnnnnX",
            "XnwwwwwwwwnX",
            "XnnnnnnnnnnX",
            "XnwwwwwwnnnX",
            "XnnnnnnnnnnX",
            ".XXXXXXXXXX.",
            ".....XnX....",
            ".....XnX....",
            ".....XnX....",
            "....XXXXX...",
        },
        // Icone dell'interfaccia (barra a sinistra): linee chiare su sfondo scuro.
        ["ui_panel"] = new[]
        {
            ".wwwwwwwwww.",
            ".w.w......w.",
            ".w.w......w.",
            ".w.w......w.",
            ".w.w......w.",
            ".w.w......w.",
            ".w.w......w.",
            ".wwwwwwwwww.",
        },
        ["ui_bag"] = new[]
        {
            "....wwww....",
            "...w....w...",
            "..wwwwwwww..",
            ".wwwwwwwwww.",
            ".wwwwwwwwww.",
            ".wsssssssww.",
            ".wsssssssww.",
            ".wwwwwwwwww.",
            ".wwwwwwwwww.",
            "..wwwwwwww..",
        },
        ["ui_menu"] = new[]
        {
            "............",
            ".wwwwwwwwww.",
            ".wwwwwwwwww.",
            "............",
            ".wwwwwwwwww.",
            ".wwwwwwwwww.",
            "............",
            ".wwwwwwwwww.",
            ".wwwwwwwwww.",
            "............",
        },
        ["box"] = new[]
        {
            "..XXXXXXXX..",
            ".XwwwwwwwwX.",
            "XwsssssssswX",
            "XwsSSSSSSswX",
            "XwsSSSSSSswX",
            "XwsssssssswX",
            ".XwwwwwwwwX.",
            "..XXXXXXXX..",
        },
    };

    static Texture2D FromColorRows(string[] rows)
    {
        int h = rows.Length, w = 0;
        foreach (var r in rows) w = Mathf.Max(w, r.Length);
        var pixels = new Color32[w * h];
        for (int row = 0; row < h; row++)
        {
            for (int col = 0; col < rows[row].Length; col++)
            {
                if (!Palette.TryGetValue(rows[row][col], out var c)) continue;
                pixels[(h - 1 - row) * w + col] = c;
            }
        }
        var tex = NewTexture(w, h);
        tex.SetPixels32(pixels);
        tex.Apply();
        return tex;
    }

    static readonly Dictionary<char, Color32> Palette = new Dictionary<char, Color32>
    {
        ['X'] = new Color32(38, 38, 46, 255),
        ['r'] = new Color32(222, 66, 66, 255), ['R'] = new Color32(150, 36, 44, 255), ['p'] = new Color32(255, 160, 160, 255),
        ['y'] = new Color32(250, 208, 70, 255), ['Y'] = new Color32(196, 142, 30, 255),
        ['b'] = new Color32(80, 150, 240, 255), ['B'] = new Color32(44, 90, 170, 255), ['c'] = new Color32(170, 220, 255, 255),
        ['n'] = new Color32(160, 108, 62, 255), ['N'] = new Color32(102, 66, 38, 255),
        ['s'] = new Color32(176, 182, 198, 255), ['S'] = new Color32(112, 118, 136, 255),
        ['w'] = new Color32(238, 238, 238, 255),
    };

    // Legenda: X contorno, w colore pieno, g chiaro, k scuro, . trasparente.
    static readonly string[] MinorArt =
    {
        "....XXXX....",
        "...XwwwwX...",
        "...XwwwwX...",
        "....XwwX....",
        "...XXwwXX...",
        "..XwwwwwwX..",
        "..XgwwwwgX..",
        "...XwwwwX...",
        "...XwwwwX...",
        "..XwwwwwwX..",
        ".XwwwwwwwwX.",
        ".XkkkkkkkkX.",
        ".XXXXXXXXXX.",
    };

    static readonly string[] ChampionArt =
    {
        "...X.XX.X...",
        "...XwXXwX...",
        "...XwwwwX...",
        "...XwwwwX...",
        "....XwwX....",
        "...XXwwXX...",
        "..XwwwwwwX..",
        "..XgwwwwgX..",
        "...XwwwwX...",
        "...XwwwwX...",
        "..XwwwwwwX..",
        ".XwwwwwwwwX.",
        ".XkkkkkkkkX.",
        ".XXXXXXXXXX.",
    };

    static readonly string[] NpcArt =
    {
        ".....XX.....",
        "....XwwX....",
        "...XXwwXX...",
        "...XwwwwX...",
        "...XwXXwX...",
        "...XwwwwX...",
        "....XXXX....",
        "...XwwwwX...",
        "..XwwwwwwX..",
        "..XwwwwwwX..",
        "..XgwwwwgX..",
        "..XwwXXwwX..",
        "..XXX..XXX..",
    };

    static readonly string[] ItemArt =
    {
        "..XXXXXXXX..",
        ".XwwwwwwwwX.",
        ".XwXXXXXXwX.",
        ".XwwwwwwwwX.",
        ".XgggwwgggX.",
        ".XwwwXXwwwX.",
        ".XwwwXXwwwX.",
        ".XkkkkkkkkX.",
        "..XXXXXXXX..",
    };

    static readonly string[] StructureArt =
    {
        "......XXX...",
        "......XwwX..",
        "......XwwX..",
        "......XXXX..",
        "......X.....",
        ".....XwX....",
        ".XXXXXwXXXX.",
        ".XwwwwwwwwX.",
        ".XwgwwwwgwX.",
        ".XwwwwwwwwX.",
        ".XwwXXXXwwX.",
        ".XwwXggXwwX.",
        ".XkkXXXXkkX.",
        ".XXXXXXXXXX.",
    };

    static float HexHalfHeight(float dx)
    {
        const float circumRadius = 18.5f;   // 32 px di larghezza / (sqrt(3) x 0.5 di schiacciamento)
        return Mathf.Abs(dx) <= 16f ? (circumRadius - Mathf.Abs(dx) / 1.7320508f) * 0.5f : -1f;
    }

    static Texture2D NewTexture(int w, int h)
    {
        return new Texture2D(w, h, TextureFormat.RGBA32, false)
        {
            filterMode = FilterMode.Point,
            wrapMode = TextureWrapMode.Clamp,
            hideFlags = HideFlags.HideAndDontSave,
        };
    }

    static Sprite Finish(Texture2D tex, Color32[] pixels, Vector2 pivot)
    {
        tex.SetPixels32(pixels);
        tex.Apply();
        return Sprite.Create(tex, new Rect(0, 0, tex.width, tex.height), pivot, PixelsPerUnit);
    }

    // Un blocco: faccia superiore (rombo o esagono schiacciato) piu' uno spessore in basso.
    // halfHeight(dx) e' la meta' dell'altezza della faccia superiore alla distanza dx dal centro
    // (negativa fuori dalla forma). Il perno e' al centro della faccia superiore.
    static Sprite BuildSlab(int w, int topHeight, int thickness, Func<float, float> halfHeight)
    {
        int h = topHeight + thickness;
        float cy = thickness + topHeight / 2f;
        var top = new bool[w, h];
        for (int px = 0; px < w; px++)
        {
            float hh = halfHeight(px + 0.5f - w / 2f);
            if (hh < 0f) continue;
            for (int py = 0; py < h; py++)
            {
                float y = py + 0.5f;
                if (y >= cy - hh && y <= cy + hh) top[px, py] = true;
            }
        }

        var pixels = new Color32[w * h];
        for (int px = 0; px < w; px++)
        {
            int lowest = -1;
            for (int py = 0; py < h; py++)
            {
                if (!top[px, py]) continue;
                if (lowest < 0) lowest = py;
                bool edge = px == 0 || py == 0 || px == w - 1 || py == h - 1
                    || !top[px - 1, py] || !top[px + 1, py] || !top[px, py - 1] || !top[px, py + 1];
                pixels[py * w + px] = Gray(edge ? 0.82f : 1f);
            }
            if (lowest < 0) continue;
            // Lo spessore: piu' scuro a sinistra, un po' meno a destra, come una luce da destra.
            for (int k = 1; k <= thickness; k++)
            {
                int py = lowest - k;
                if (py < 0) break;
                float shade = px < w / 2 ? 0.52f : 0.68f;
                pixels[py * w + px] = Gray(k == thickness ? shade * 0.8f : shade);
            }
        }
        return Finish(NewTexture(w, h), pixels, new Vector2(0.5f, cy / h));
    }

    static Color32 Gray(float v)
    {
        byte b = (byte)Mathf.Clamp(Mathf.RoundToInt(v * 255f), 0, 255);
        return new Color32(b, b, b, 255);
    }

    static Sprite BuildEllipse(int w, int h, bool outlineOnly, Color color)
    {
        var pixels = new Color32[w * h];
        for (int py = 0; py < h; py++)
        {
            for (int px = 0; px < w; px++)
            {
                if (!InEllipse(px, py, w, h, 0f)) continue;
                if (outlineOnly && InEllipse(px, py, w, h, 1.2f)) continue;
                pixels[py * w + px] = color;
            }
        }
        return Finish(NewTexture(w, h), pixels, new Vector2(0.5f, 0.5f));
    }

    static bool InEllipse(int px, int py, int w, int h, float shrink)
    {
        float rx = w / 2f - shrink, ry = h / 2f - shrink * h / w;
        if (rx <= 0f || ry <= 0f) return false;
        float dx = (px + 0.5f - w / 2f) / rx, dy = (py + 0.5f - h / 2f) / ry;
        return dx * dx + dy * dy <= 1f;
    }

    // Righe di testo -> sprite. La prima riga e' in alto; il perno e' in basso al centro (i piedi).
    static Sprite FromRows(string[] rows)
    {
        int h = rows.Length, w = 0;
        foreach (var r in rows) w = Mathf.Max(w, r.Length);
        var pixels = new Color32[w * h];
        for (int row = 0; row < h; row++)
        {
            for (int col = 0; col < rows[row].Length; col++)
            {
                Color32 c;
                switch (rows[row][col])
                {
                    case 'X': c = new Color32(38, 38, 46, 255); break;
                    case 'w': c = Gray(1f); break;
                    case 'g': c = Gray(0.82f); break;
                    case 'k': c = Gray(0.5f); break;
                    default: continue;
                }
                pixels[(h - 1 - row) * w + col] = c;
            }
        }
        return Finish(NewTexture(w, h), pixels, new Vector2(0.5f, 0f));
    }
}
