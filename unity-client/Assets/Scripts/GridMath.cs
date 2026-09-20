using UnityEngine;

// Geometria della griglia: caselle quadrate viste dall'alto, una unita' di scena per casella.
// La casella (0,0) e' in alto a sinistra e la y cresce verso il basso (come le righe del
// terreno), quindi in scena la coordinata verticale e' -riga. Stessa logica del backend
// (internal/game/grid.go): 8 vicini, distanza di Chebyshev.
public static class GridMath
{
    // I piedi di una pedina stanno un po' sotto il centro della casella, cosi' la testa resta dentro.
    public const float FeetOffset = -0.32f;

    // Numero di passi tra due caselle (mosse del re).
    public static int Distance(int ax, int ay, int bx, int by) => Mathf.Max(Mathf.Abs(ax - bx), Mathf.Abs(ay - by));

    // Centro della casella in coordinate di scena.
    public static Vector2 CellToWorld(int x, int y) => new Vector2(x + 0.5f, -(y + 0.5f));

    // La casella che contiene il punto (puo' cadere fuori dalla board).
    public static Vector2Int WorldToCell(Vector2 p) => new Vector2Int(Mathf.FloorToInt(p.x), Mathf.FloorToInt(-p.y));

    // Rettangolo di scena che contiene tutta la board (per inquadrarla).
    public static Rect Bounds(int width, int height) => new Rect(0f, -height, width, height);
}
