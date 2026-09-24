using System;
using System.Collections.Generic;
using UnityEngine;

// Sprite in pixel art disegnati in codice (pedine, ombre, anelli, icone degli oggetti); il terreno
// e' in TileSheet.cs, con i tile di Kenney.
// Gli sprite sono in scala di grigi (bianco = colore pieno) e si colorano con SpriteRenderer.color,
// cosi' lo stesso disegno serve a tutte le squadre. Le texture usano il filtro Point (niente sfocatura).
public static class PixelArt
{
    // Un'unita' di scena e' una casella, larga 16 pixel come i tile del terreno.
    public const int PixelsPerUnit = 16;

    static Sprite shadow, teamBase, ring, portal;
    static readonly Dictionary<string, Sprite> pieces = new Dictionary<string, Sprite>();

    // Disegna uno sprite nell'interfaccia (IMGUI): la sua parte della texture, non tutta la tavola.
    public static void DrawSprite(Rect rect, Sprite sprite, Color tint)
    {
        if (sprite == null) return;
        var t = sprite.texture;
        var r = sprite.textureRect;
        var previous = GUI.color;
        GUI.color = tint;
        GUI.DrawTextureWithTexCoords(rect, t, new Rect(r.x / t.width, r.y / t.height, r.width / t.width, r.height / t.height));
        GUI.color = previous;
    }

    public static Sprite Shadow => shadow ??= BuildEllipse(12, 5, false, new Color(0f, 0f, 0f, 0.35f));
    // La base colorata sotto i piedi: dice a quale squadra appartiene una pedina.
    public static Sprite TeamBase => teamBase ??= BuildEllipse(16, 7, false, Color.white);
    public static Sprite Ring => ring ??= BuildEllipse(16, 8, true, new Color(1f, 0.92f, 0.25f, 1f));
    public static Sprite Portal => portal ??= BuildEllipse(14, 7, true, new Color(0.85f, 0.55f, 1f, 1f));

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

    static readonly Dictionary<string, Sprite> itemSprites = new Dictionary<string, Sprite>();

    // L'icona di un oggetto come sprite, per l'oggetto a terra: a colori veri, con i piedi in basso.
    public static Sprite ItemSprite(string key)
    {
        var tex = ItemIcon(key);
        if (!itemSprites.TryGetValue(tex.name, out var sprite))
            sprite = itemSprites[tex.name] = Sprite.Create(tex, new Rect(0, 0, tex.width, tex.height), new Vector2(0.5f, 0f), PixelsPerUnit);
        return sprite;
    }

    public static Texture2D ItemIcon(string key)
    {
        if (string.IsNullOrEmpty(key) || !IconArt.ContainsKey(key)) key = "box";
        if (!itemIcons.TryGetValue(key, out var tex))
        {
            tex = itemIcons[key] = FromColorRows(IconArt[key]);
            tex.name = key;
        }
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
        ["ui_star"] = new[]
        {
            ".....XX.....",
            "....XyyX....",
            "....XyyX....",
            "XXXXXyyXXXXX",
            "XyyyyyyyyyyX",
            ".XyyyyyyyyX.",
            "..XyyyyyyX..",
            "..XyyyyyyX..",
            ".XyyyXXyyyX.",
            ".XyyX..XyyX.",
            "XyyX....XyyX",
            "XXX......XXX",
        },
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
