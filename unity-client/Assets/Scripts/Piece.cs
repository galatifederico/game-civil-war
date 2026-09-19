using UnityEngine;

public enum PieceKind { Champion, Soldier, Npc, Object }

public class Piece : MonoBehaviour
{
    public int X;
    public int Z;
    public PieceKind Kind;
    public string DisplayName;
    public string Description;

    Renderer rend;
    Color baseColor;

    public bool Movable => Kind == PieceKind.Champion || Kind == PieceKind.Soldier;

    public string KindLabel
    {
        get
        {
            switch (Kind)
            {
                case PieceKind.Champion: return "Champion - la tua squadra";
                case PieceKind.Soldier: return "Pedina - la tua squadra";
                case PieceKind.Npc: return "NPC - non controllabile";
                default: return "Oggetto - non controllabile";
            }
        }
    }

    void Awake()
    {
        rend = GetComponent<Renderer>();
        baseColor = rend.material.color;
    }

    void OnMouseDown()
    {
        BoardManager.Instance.OnPieceClicked(this);
    }

    public void SetHighlight(bool on)
    {
        rend.material.color = on ? Color.Lerp(baseColor, Color.white, 0.5f) : baseColor;
    }

    public void MoveTo(int x, int z)
    {
        X = x;
        Z = z;
        transform.position = new Vector3(x * BoardManager.CellSize, transform.position.y, z * BoardManager.CellSize);
    }
}
