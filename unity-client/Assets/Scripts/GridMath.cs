using UnityEngine;

// Geometria delle griglie (stessa logica del backend, internal/game/grid.go) e passaggio da
// coordinate di cella a coordinate di scena. La vista e' isometrica: una casella quadrata e' un
// rombo 2:1, una casella esagonale e' un esagono "schiacciato" con lo stesso rapporto.
public static class GridMath
{
    public const string Square = "square";
    public const string Hex = "hex";

    public const float TileWidth = 1f;
    const float SquareRowStep = 0.25f;           // mezzo rombo in altezza (larghezza 1, altezza 0.5)
    const float HexRowStep = 0.433f;             // 0.866 (passo tra righe di esagoni) x 0.5 (schiacciamento)
    const float HexSquash = 0.5f;

    // Numero di passi tra due caselle (quadrata: mosse del re; esagonale: distanza esagonale).
    public static int Distance(string grid, int ax, int ay, int bx, int by)
    {
        if (grid == Hex)
        {
            int aq = ax - (ay - (ay & 1)) / 2, bq = bx - (by - (by & 1)) / 2;
            int dq = aq - bq, dr = ay - by;
            return Mathf.Max(Mathf.Abs(dq), Mathf.Abs(dr), Mathf.Abs(dq + dr));
        }
        return Mathf.Max(Mathf.Abs(ax - bx), Mathf.Abs(ay - by));
    }

    // Centro della casella in coordinate di scena.
    public static Vector2 CellToWorld(string grid, int x, int y)
    {
        if (grid == Hex) return new Vector2((x + 0.5f * (y & 1)) * TileWidth, y * HexRowStep);
        return new Vector2((x - y) * 0.5f * TileWidth, (x + y) * SquareRowStep);
    }

    // La casella il cui centro e' piu' vicino al punto (puo' cadere fuori dalla board).
    public static Vector2Int WorldToCell(string grid, Vector2 p)
    {
        if (grid == Hex)
        {
            int row = Mathf.RoundToInt(p.y / HexRowStep);
            var best = new Vector2Int(0, row);
            float bestDist = float.MaxValue;
            for (int r = row - 1; r <= row + 1; r++)
            {
                int col = Mathf.RoundToInt(p.x / TileWidth - 0.5f * (r & 1));
                for (int c = col - 1; c <= col + 1; c++)
                {
                    var centre = CellToWorld(grid, c, r);
                    float dx = centre.x - p.x, dy = (centre.y - p.y) / HexSquash;
                    float d = dx * dx + dy * dy;
                    if (d < bestDist) { bestDist = d; best = new Vector2Int(c, r); }
                }
            }
            return best;
        }
        float u = p.x / (0.5f * TileWidth);
        float v = p.y / SquareRowStep;
        return new Vector2Int(Mathf.RoundToInt((u + v) / 2f), Mathf.RoundToInt((v - u) / 2f));
    }

    // Rettangolo di scena che contiene tutta la board (per inquadrarla).
    public static Rect Bounds(string grid, int width, int height)
    {
        float minX = float.MaxValue, minY = float.MaxValue, maxX = float.MinValue, maxY = float.MinValue;
        foreach (var corner in new[] { (0, 0), (width - 1, 0), (0, height - 1), (width - 1, height - 1) })
        {
            var c = CellToWorld(grid, corner.Item1, corner.Item2);
            minX = Mathf.Min(minX, c.x); maxX = Mathf.Max(maxX, c.x);
            minY = Mathf.Min(minY, c.y); maxY = Mathf.Max(maxY, c.y);
        }
        // Margine di mezza casella (e lo spessore del "blocco" sotto).
        return Rect.MinMaxRect(minX - 0.6f, minY - 0.6f, maxX + 0.6f, maxY + 0.5f);
    }
}
