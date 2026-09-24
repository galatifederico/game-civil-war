using System;
using System.Collections.Generic;
using UnityEngine;

// Gli sprite dei personaggi. C'e' una sola pedina standard (una creatura tonda e paffuta, generata
// con Pixellab a partire da img/pawn.png); tutte le altre sono quella pedina con qualcosa di
// cambiato: la razza le da' un colore, il ruolo aggiunge un accessorio (il campione ha la corona),
// gli NPC hanno colori e copricapo loro. Le varianti si costruiscono con tools/sprites/build_characters.py.
// Stanno in un'unica tavola letta a runtime da Resources/Art/characters.bytes (un PNG rinominato)
// con la descrizione in characters-layout.json: per ogni variante una riga di celle con 4 pose
// ferme (sud, est, nord, ovest) e, se cammina, alcuni fotogrammi per direzione. Il nome della
// variante arriva dal server (game.UnitSprite, game.SpriteChoices).
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

    // Un'unita' di scena e' una casella (16 px di terreno): la pedina, larga 28 px, la riempie.
    public const int PixelsPerUnit = 28;

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

    // Quale sprite disegna una entita': quello che dice il server (la variante della razza e del ruolo
    // per le pedine, lo sprite scelto per un NPC); se la tavola non lo ha, quello del suo ruolo.
    public static string NameFor(EntityData e)
    {
        string role;
        switch (e.kind)
        {
            case Kinds.Champion: role = "champion"; break;
            case Kinds.Minor: role = "minor"; break;
            case Kinds.Npc: role = "wanderer"; break;
            default: return null;
        }
        if (!string.IsNullOrEmpty(e.sprite) && Has(e.sprite)) return e.sprite;
        return role;
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
        return sprites[key] = Sprite.Create(texture, rect, pivot, PixelsPerUnit);
    }
}
