using UnityEngine;

public class Piece : MonoBehaviour
{
    public int X;
    public int Z;

    Renderer rend;
    Color baseColor;

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
