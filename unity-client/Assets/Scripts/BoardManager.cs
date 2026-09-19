using System.Collections.Generic;
using UnityEngine;

// Mostra la board e le pedine che il server descrive (snapshot + delta) e traduce i click
// in comandi: la pedina si muove solo quando il server conferma.
public class BoardManager : MonoBehaviour
{
    public const float CellSize = 1f;
    const float BoardTop = 0.05f;

    public static BoardManager Instance { get; private set; }

    NetworkClient net;
    InfoPanel panel;
    Camera cam;

    readonly Dictionary<string, Piece> pieces = new Dictionary<string, Piece>();
    readonly List<GameObject> squareObjects = new List<GameObject>();
    Material lightMaterial, darkMaterial;
    Piece selected;
    string myPlayerId;
    int width, height;
    float fittedAspect;

    public static Vector3 CellToWorld(int x, int y, float height) => new Vector3(x * CellSize, height, y * CellSize);

    void Awake() => Instance = this;

    public void Init(NetworkClient network, InfoPanel infoPanel, Camera camera)
    {
        net = network;
        panel = infoPanel;
        cam = camera;
    }

    public void LoadSnapshot(ServerMessage snapshot)
    {
        Clear();
        myPlayerId = snapshot.your_player_id;
        width = snapshot.board.width;
        height = snapshot.board.height;
        BuildSquares();
        FitCamera();
        foreach (var e in snapshot.entities) Spawn(e);
    }

    public void ApplyDelta(ServerMessage delta)
    {
        foreach (var e in delta.entities ?? new EntityData[0])
        {
            if (pieces.TryGetValue(e.id, out var piece)) piece.Apply(e);
            else Spawn(e);
        }
        foreach (var id in delta.removed ?? new string[0])
        {
            if (!pieces.TryGetValue(id, out var piece)) continue;
            if (piece == selected) ClearSelection();
            pieces.Remove(id);
            Destroy(piece.gameObject);
        }
    }

    public void Clear()
    {
        ClearSelection();
        foreach (var p in pieces.Values) Destroy(p.gameObject);
        pieces.Clear();
        foreach (var s in squareObjects) Destroy(s);
        squareObjects.Clear();
    }

    void BuildSquares()
    {
        if (lightMaterial == null)
        {
            lightMaterial = NewMaterial(new Color(0.85f, 0.85f, 0.75f));
            darkMaterial = NewMaterial(new Color(0.35f, 0.25f, 0.2f));
        }
        for (int x = 0; x < width; x++)
        {
            for (int y = 0; y < height; y++)
            {
                var cube = GameObject.CreatePrimitive(PrimitiveType.Cube);
                cube.name = $"Square_{x}_{y}";
                cube.transform.SetParent(transform);
                cube.transform.position = CellToWorld(x, y, 0f);
                cube.transform.localScale = new Vector3(CellSize * 0.95f, 0.1f, CellSize * 0.95f);
                cube.GetComponent<Renderer>().sharedMaterial = (x + y) % 2 == 0 ? lightMaterial : darkMaterial;
                var square = cube.AddComponent<Square>();
                square.X = x;
                square.Z = y;
                squareObjects.Add(cube);
            }
        }
    }

    // Stesso shader delle primitive: funziona con qualunque render pipeline del progetto.
    static Material NewMaterial(Color color)
    {
        var probe = GameObject.CreatePrimitive(PrimitiveType.Cube);
        var material = new Material(probe.GetComponent<Renderer>().sharedMaterial) { color = color };
        Destroy(probe);
        material.SetFloat("_Glossiness", 0f);
        return material;
    }

    void FitCamera()
    {
        fittedAspect = cam.aspect;
        cam.orthographicSize = Mathf.Max(height * CellSize * 0.5f + 1f, (width * CellSize * 0.5f + 1f) / cam.aspect);
        cam.transform.position = new Vector3((width - 1) * CellSize / 2f, 10f, (height - 1) * CellSize / 2f);
    }

    void LateUpdate()
    {
        if (width > 0 && !Mathf.Approximately(fittedAspect, cam.aspect)) FitCamera();
    }

    void Spawn(EntityData e)
    {
        bool mine = !string.IsNullOrEmpty(e.owner_id) && e.owner_id == myPlayerId;
        Look(e, mine, out var shape, out var scale, out var halfHeight, out var color);

        var go = GameObject.CreatePrimitive(shape);
        go.name = $"{e.kind}_{e.name}";
        go.transform.SetParent(transform);
        go.transform.localScale = scale;
        go.transform.position = CellToWorld(e.x, e.y, BoardTop + scale.y * halfHeight);

        var piece = go.AddComponent<Piece>();
        piece.Init(e, mine, color);
        pieces[e.id] = piece;
    }

    static void Look(EntityData e, bool mine, out PrimitiveType shape, out Vector3 scale, out float halfHeight, out Color color)
    {
        switch (e.kind)
        {
            case Kinds.Champion:
                shape = PrimitiveType.Cylinder; scale = new Vector3(0.7f, 0.55f, 0.7f); halfHeight = 1f;
                color = mine ? new Color(1f, 0.8f, 0.15f) : OwnerColor(e.owner_id, 0.95f);
                break;
            case Kinds.Minor:
                shape = PrimitiveType.Capsule; scale = new Vector3(0.55f, 0.45f, 0.55f); halfHeight = 1f;
                color = mine ? new Color(0.2f, 0.45f, 1f) : OwnerColor(e.owner_id, 0.75f);
                break;
            case Kinds.Npc:
                shape = PrimitiveType.Sphere; scale = new Vector3(0.6f, 0.6f, 0.6f); halfHeight = 0.5f;
                color = new Color(0.7f, 0.3f, 0.85f);
                break;
            default:
                shape = PrimitiveType.Cube; scale = new Vector3(0.55f, 0.4f, 0.55f); halfHeight = 0.5f;
                color = new Color(0.45f, 0.75f, 0.5f);
                break;
        }
    }

    // Ogni squadra avversaria ha il proprio colore, stabile tra un avvio e l'altro.
    static Color OwnerColor(string ownerId, float value)
    {
        int hash = 17;
        foreach (char c in ownerId) hash = hash * 31 + c;
        float hue = (hash & 0xFFFF) / 65535f;
        // Il blu e' riservato alla tua squadra: salta la fascia di tinte vicine.
        hue = (hue * 0.55f + 0.85f) % 1f;
        return Color.HSVToRGB(hue, 0.7f, value);
    }

    public void OnPieceClicked(Piece piece)
    {
        if (panel.BlocksPointer) return;

        if (piece == selected)
        {
            ClearSelection();
            panel.Hide();
            return;
        }

        ClearSelection();
        if (piece.Movable)
        {
            selected = piece;
            piece.SetHighlight(true);
        }
        panel.Show(piece);
    }

    public void OnSquareClicked(Square square)
    {
        if (panel.BlocksPointer) return;

        if (selected == null)
        {
            panel.Hide();
            return;
        }
        net.SendMove(selected.Data.id, square.X, square.Z);
    }

    public void ClearSelection()
    {
        if (selected != null) selected.SetHighlight(false);
        selected = null;
    }
}
