using UnityEngine;

public class Square : MonoBehaviour
{
    public int X;
    public int Z;
    public Piece OccupiedBy;

    void OnMouseDown()
    {
        BoardManager.Instance.OnSquareClicked(this);
    }
}
