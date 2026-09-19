using UnityEngine;

// Rappresentazione di un'entita' del server (pedina, NPC, oggetto). Non decide nulla:
// applica lo stato ricevuto e scorre visivamente verso la casella indicata.
public class Piece : MonoBehaviour
{
    const float SlideSpeed = 10f;

    public EntityData Data { get; private set; }
    public bool Mine { get; private set; }

    Renderer rend;
    Color baseColor;
    Vector3 target;
    float readyAt;

    public bool Movable => Mine && (Data.kind == Kinds.Champion || Data.kind == Kinds.Minor);
    public bool IsUnit => Data.kind != Kinds.Item;
    public float SecondsUntilReady => Mathf.Max(0f, readyAt - Time.time);

    public string KindLabel
    {
        get
        {
            switch (Data.kind)
            {
                case Kinds.Champion: return Mine ? "Champion - la tua squadra" : "Champion - squadra avversaria";
                case Kinds.Minor: return Mine ? "Pedina - la tua squadra" : "Pedina - squadra avversaria";
                case Kinds.Npc: return "NPC - non controllabile";
                default: return "Oggetto - non controllabile";
            }
        }
    }

    public void Init(EntityData data, bool mine, Color color)
    {
        rend = GetComponent<Renderer>();
        rend.material.color = color;
        baseColor = color;
        Mine = mine;
        Apply(data);
        transform.position = target;
    }

    public void Apply(EntityData data)
    {
        Data = data;
        target = BoardManager.CellToWorld(data.x, data.y, transform.position.y);
        readyAt = Time.time + data.ready_in_ms / 1000f;
    }

    void Update()
    {
        transform.position = Vector3.MoveTowards(transform.position, target, SlideSpeed * Time.deltaTime);
    }

    void OnMouseDown()
    {
        BoardManager.Instance.OnPieceClicked(this);
    }

    public void SetHighlight(bool on)
    {
        rend.material.color = on ? Color.Lerp(baseColor, Color.white, 0.5f) : baseColor;
    }
}
