using System;
using System.Collections.Generic;
using UnityEngine;

// Gli sprite dei personaggi (campione, pedine, NPC): originali, generati con Pixellab (vista
// dall'alto 3/4, 4 direzioni, ~20 px di altezza). Stanno in un'unica tavola letta a runtime da
// Resources/Art/characters.bytes (un PNG rinominato) con la descrizione in characters-layout.json:
// per ogni personaggio una riga di celle 28x28 con 4 pose ferme (sud, est, nord, ovest) e, se
// cammina, 4 fotogrammi per direzione. Il nome dello sprite arriva dal server (game.SpriteNames).
public static class CharacterSprites
{
    [Serializable]
    class Entry
    {
        public string name;
        public int row;
        public bool walk;
    }

    [Serializable]
    class Layout
    {
        public int cell;
        public int feet;
        public int walkFrames;
        public Entry[] characters;
    }

    // Sud, est, nord, ovest: l'ordine delle celle nella tavola.
    public const int South = 0, East = 1, North = 2, West = 3;

    static Texture2D texture;
    static Layout layout;
    static bool triedToLoad;
    static readonly Dictionary<string, Sprite> sprites = new Dictionary<string, Sprite>();

    static bool Load()
    {
        if (triedToLoad) return layout != null;
        triedToLoad = true;
        var png = Resources.Load<TextAsset>("Art/characters");
        var json = Resources.Load<TextAsset>("Art/characters-layout");
        if (png == null || json == null)
        {
            Debug.LogWarning("CharacterSprites: mancano Resources/Art/characters.bytes o characters-layout.json.");
            return false;
        }
        texture = new Texture2D(2, 2, TextureFormat.RGBA32, false) { filterMode = FilterMode.Point, wrapMode = TextureWrapMode.Clamp, hideFlags = HideFlags.HideAndDontSave };
        texture.LoadImage(png.bytes);
        layout = JsonUtility.FromJson<Layout>(json.text);
        return true;
    }

    // Quale sprite disegna una entita': quello scelto dal server per un NPC, altrimenti quello del suo ruolo.
    public static string NameFor(EntityData e)
    {
        switch (e.kind)
        {
            case Kinds.Champion: return "champion";
            case Kinds.Minor: return "minor";
            case Kinds.Npc: return string.IsNullOrEmpty(e.sprite) ? "wanderer" : e.sprite;
            default: return null;
        }
    }

    static Entry Find(string name)
    {
        if (name == null || !Load()) return null;
        foreach (var c in layout.characters)
            if (c.name == name) return c;
        return null;
    }

    public static bool Has(string name) => Find(name) != null;

    public static bool CanWalk(string name) => Find(name)?.walk == true;

    public static int WalkFrames => layout != null ? layout.walkFrames : 4;

    // La posa ferma verso una direzione.
    public static Sprite Idle(string name, int direction) => Cell(name, direction);

    // Un fotogramma della camminata (se il personaggio non cammina, la posa ferma).
    public static Sprite Walk(string name, int direction, int frame)
    {
        var entry = Find(name);
        if (entry == null || !entry.walk) return Idle(name, direction);
        return Cell(name, 4 + direction * layout.walkFrames + frame % layout.walkFrames);
    }

    static Sprite Cell(string name, int column)
    {
        var entry = Find(name);
        if (entry == null) return null;
        string key = name + ":" + column;
        if (sprites.TryGetValue(key, out var sprite)) return sprite;
        var rect = new Rect(column * layout.cell, texture.height - (entry.row + 1) * layout.cell, layout.cell, layout.cell);
        // Il perno e' ai piedi (la riga dei piedi e' nella descrizione), cosi' la pedina sta sulla casella.
        var pivot = new Vector2(0.5f, (layout.cell - layout.feet) / (float)layout.cell);
        return sprites[key] = Sprite.Create(texture, rect, pivot, PixelArt.PixelsPerUnit);
    }
}
