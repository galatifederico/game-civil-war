using UnityEngine;

public class Square : MonoBehaviour
{
    public int X;
    public int Z;

    // Le pedine hanno un collider sopra la casella: se il click arriva qui, la casella e' libera.
    void OnMouseDown()
    {
        BoardManager.Instance.OnSquareClicked(this);
    }
}
