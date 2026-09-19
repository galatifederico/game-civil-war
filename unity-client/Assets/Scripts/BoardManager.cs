using UnityEngine;

// Costruisce scacchiera e pedine a runtime (nessun setup manuale nella scena) e gestisce i click:
// ogni pedina apre il pannello con la descrizione, ma solo quelle della propria squadra si muovono.
public class BoardManager : MonoBehaviour
{
    public const int Size = 10;
    public const float CellSize = 1f;
    const float BoardTop = 0.05f;

    public static BoardManager Instance { get; private set; }

    readonly Square[,] squares = new Square[Size, Size];
    Piece selectedPiece;
    InfoPanel panel;

    [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.AfterSceneLoad)]
    static void Bootstrap()
    {
        if (Instance != null) return;
        new GameObject("BoardManager").AddComponent<BoardManager>();
    }

    void Awake()
    {
        Instance = this;
        SetupCamera();
        BuildBoard();
        SpawnPieces();
        panel = gameObject.AddComponent<InfoPanel>();
        panel.Closed += Deselect;
    }

    void SetupCamera()
    {
        var camGO = Camera.main != null ? Camera.main.gameObject : new GameObject("Main Camera");
        if (camGO.GetComponent<Camera>() == null)
        {
            camGO.AddComponent<Camera>();
            camGO.tag = "MainCamera";
        }
        if (camGO.GetComponent<AudioListener>() == null)
            camGO.AddComponent<AudioListener>();

        var cam = camGO.GetComponent<Camera>();
        cam.orthographic = true;
        cam.orthographicSize = Size * CellSize * 0.6f;
        cam.clearFlags = CameraClearFlags.SolidColor;
        cam.backgroundColor = new Color(0.1f, 0.1f, 0.12f);
        camGO.transform.position = new Vector3((Size - 1) * CellSize / 2f, 10f, (Size - 1) * CellSize / 2f);
        camGO.transform.rotation = Quaternion.Euler(90f, 0f, 0f);

        var lightGO = new GameObject("Directional Light");
        lightGO.transform.SetParent(transform);
        lightGO.transform.rotation = Quaternion.Euler(90f, 0f, 0f);
        lightGO.AddComponent<Light>().type = LightType.Directional;
    }

    void BuildBoard()
    {
        for (int x = 0; x < Size; x++)
        {
            for (int z = 0; z < Size; z++)
            {
                var cube = GameObject.CreatePrimitive(PrimitiveType.Cube);
                cube.name = $"Square_{x}_{z}";
                cube.transform.SetParent(transform);
                cube.transform.position = new Vector3(x * CellSize, 0f, z * CellSize);
                cube.transform.localScale = new Vector3(CellSize * 0.95f, 0.1f, CellSize * 0.95f);

                bool light = (x + z) % 2 == 0;
                var mat = cube.GetComponent<Renderer>().material;
                mat.color = light ? new Color(0.85f, 0.85f, 0.75f) : new Color(0.35f, 0.25f, 0.2f);
                mat.SetFloat("_Glossiness", 0f);

                var square = cube.AddComponent<Square>();
                square.X = x;
                square.Z = z;
                squares[x, z] = square;
            }
        }
    }

    void SpawnPieces()
    {
        // Squadra del giocatore: 1 champion + 12 pedine.
        SpawnPiece(4, 1, PieceKind.Champion, "Champion",
            "Il campione della tua squadra: forte, carismatico e convinto di essere indispensabile. Puoi muoverlo.");
        int n = 0;
        for (int x = 0; x < Size; x++) SpawnSoldier(x, 0, ++n);
        SpawnSoldier(3, 1, ++n);
        SpawnSoldier(5, 1, ++n);

        // NPC: non controllabili.
        SpawnPiece(1, 8, PieceKind.Npc, "Mercante",
            "Vende merci di dubbia provenienza a prezzi ancora piu' dubbi. Non si allontana dal suo banco.");
        SpawnPiece(4, 7, PieceKind.Npc, "Guardia",
            "Sorveglia la piazza con aria annoiata. Non e' nella tua squadra e non prende ordini da te.");
        SpawnPiece(8, 8, PieceKind.Npc, "Vecchio saggio",
            "Ha una risposta per tutto, quasi mai a una domanda che gli hai fatto.");
        SpawnPiece(7, 5, PieceKind.Npc, "Fabbro",
            "Ripara armi e armature. Sostiene che il martello sia sempre 'quasi pronto'.");
        SpawnPiece(2, 5, PieceKind.Npc, "Viandante",
            "Di passaggio, come sempre. Nessuno sa da dove venga ne' dove stia andando.");

        // Oggetti: non controllabili.
        SpawnPiece(3, 4, PieceKind.Object, "Forziere",
            "Chiuso a chiave. Nessuno ricorda dove sia finita la chiave.");
        SpawnPiece(6, 4, PieceKind.Object, "Cristallo",
            "Emette un debole bagliore. Meglio non toccarlo.");
        SpawnPiece(9, 3, PieceKind.Object, "Cartello",
            "Recita: 'Lavori in corso'. I lavori non sono mai iniziati.");
    }

    void SpawnSoldier(int x, int z, int number)
    {
        SpawnPiece(x, z, PieceKind.Soldier, $"Pedina {number}",
            "Una fedele pedina della tua squadra. Puoi muoverla.");
    }

    static (PrimitiveType shape, Vector3 scale, float halfHeight, Color color) Look(PieceKind kind)
    {
        switch (kind)
        {
            case PieceKind.Champion:
                return (PrimitiveType.Cylinder, new Vector3(0.7f, 0.55f, 0.7f), 1f, new Color(1f, 0.8f, 0.15f));
            case PieceKind.Soldier:
                return (PrimitiveType.Capsule, new Vector3(0.55f, 0.45f, 0.55f), 1f, new Color(0.2f, 0.45f, 1f));
            case PieceKind.Npc:
                return (PrimitiveType.Sphere, new Vector3(0.6f, 0.6f, 0.6f), 0.5f, new Color(0.7f, 0.3f, 0.85f));
            default:
                return (PrimitiveType.Cube, new Vector3(0.55f, 0.4f, 0.55f), 0.5f, new Color(0.45f, 0.75f, 0.5f));
        }
    }

    void SpawnPiece(int x, int z, PieceKind kind, string displayName, string description)
    {
        var look = Look(kind);
        var go = GameObject.CreatePrimitive(look.shape);
        go.name = $"Piece_{displayName}";
        go.transform.SetParent(transform);
        go.transform.localScale = look.scale;
        go.transform.position = new Vector3(x * CellSize, BoardTop + look.scale.y * look.halfHeight, z * CellSize);
        go.GetComponent<Renderer>().material.color = look.color;

        var piece = go.AddComponent<Piece>();
        piece.X = x;
        piece.Z = z;
        piece.Kind = kind;
        piece.DisplayName = displayName;
        piece.Description = description;
        squares[x, z].OccupiedBy = piece;
    }

    public void OnPieceClicked(Piece piece)
    {
        if (panel.BlocksPointer) return;

        if (piece == selectedPiece)
        {
            Deselect();
            panel.Hide();
            return;
        }

        Deselect();
        if (piece.Movable)
        {
            selectedPiece = piece;
            piece.SetHighlight(true);
        }
        panel.Show(piece);
    }

    public void OnSquareClicked(Square square)
    {
        if (panel.BlocksPointer) return;

        if (selectedPiece == null)
        {
            panel.Hide();
            return;
        }
        if (square.OccupiedBy != null) return;

        squares[selectedPiece.X, selectedPiece.Z].OccupiedBy = null;
        square.OccupiedBy = selectedPiece;
        selectedPiece.MoveTo(square.X, square.Z);
        Deselect();
    }

    void Deselect()
    {
        if (selectedPiece != null) selectedPiece.SetHighlight(false);
        selectedPiece = null;
    }
}
