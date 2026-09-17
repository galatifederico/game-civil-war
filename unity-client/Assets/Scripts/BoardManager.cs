using UnityEngine;

// Costruisce la scacchiera e le pedine a runtime (nessun setup manuale nella scena richiesto)
// e gestisce la selezione/movimento: click su una pedina per selezionarla, click su una
// casella vuota per spostarla li'.
public class BoardManager : MonoBehaviour
{
    public const int Size = 8;
    public const float CellSize = 1f;

    public static BoardManager Instance { get; private set; }

    readonly Square[,] squares = new Square[Size, Size];
    Piece selectedPiece;

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
        cam.backgroundColor = new Color(0.1f, 0.1f, 0.12f);
        camGO.transform.position = new Vector3((Size - 1) * CellSize / 2f, 10f, (Size - 1) * CellSize / 2f);
        camGO.transform.rotation = Quaternion.Euler(90f, 0f, 0f);
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
                cube.GetComponent<Renderer>().material.color =
                    light ? new Color(0.85f, 0.85f, 0.75f) : new Color(0.35f, 0.25f, 0.2f);

                var square = cube.AddComponent<Square>();
                square.X = x;
                square.Z = z;
                squares[x, z] = square;
            }
        }
    }

    void SpawnPieces()
    {
        SpawnPiece(1, 1, Color.red);
        SpawnPiece(3, 1, Color.red);
        SpawnPiece(6, 6, Color.blue);
        SpawnPiece(4, 6, Color.blue);
    }

    void SpawnPiece(int x, int z, Color color)
    {
        var capsule = GameObject.CreatePrimitive(PrimitiveType.Capsule);
        capsule.name = $"Piece_{x}_{z}";
        capsule.transform.SetParent(transform);
        capsule.transform.position = new Vector3(x * CellSize, 0.5f, z * CellSize);
        capsule.transform.localScale = new Vector3(0.6f, 0.5f, 0.6f);
        capsule.GetComponent<Renderer>().material.color = color;

        var piece = capsule.AddComponent<Piece>();
        piece.X = x;
        piece.Z = z;
        squares[x, z].OccupiedBy = piece;
    }

    public void OnPieceClicked(Piece piece)
    {
        if (selectedPiece == piece)
        {
            Deselect();
            return;
        }
        Deselect();
        selectedPiece = piece;
        selectedPiece.SetHighlight(true);
    }

    public void OnSquareClicked(Square square)
    {
        if (selectedPiece == null || square.OccupiedBy != null) return;

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
