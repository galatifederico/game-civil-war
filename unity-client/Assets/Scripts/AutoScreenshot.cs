using System.Collections;
using UnityEngine;

/// <summary>
/// Command line helper for automated checks: <c>--screenshot=path.png [--shot-after=8]</c> saves a
/// screenshot after some seconds and quits. <c>--open=team</c> (or mappa, inventario, piccione, …) opens a
/// HUD window first, <c>--walk=RRDDL</c> walks the champion (R, L, U, D: one step each),
/// <c>--select-near</c> then opens the actions with the pawn next to it;
/// <c>--open=scheda</c> opens the card of the selection, <c>--open=oggetto:birra</c> an item.
/// </summary>
public class AutoScreenshot : MonoBehaviour
{
    public string path;
    public float after = 8f;

    public static void InstallFromArgs(GameObject host)
    {
        string path = null;
        float after = 8f;
        foreach (var a in System.Environment.GetCommandLineArgs())
        {
            if (a.StartsWith("--screenshot=")) path = a.Substring(13);
            if (a.StartsWith("--shot-after=")) float.TryParse(a.Substring(13), System.Globalization.NumberStyles.Float, System.Globalization.CultureInfo.InvariantCulture, out after);
        }
        if (path == null) return;
        var s = host.AddComponent<AutoScreenshot>();
        s.path = path;
        s.after = after;
    }

    IEnumerator Start()
    {
        var view = GetComponent<SimView>();
        float t0 = Time.realtimeSinceStartup;
        while (view != null && !view.ChampionId.HasValue && Time.realtimeSinceStartup - t0 < after) yield return null;
        if (view != null && view.ChampionId.HasValue)
        {
            view.Select(view.ChampionId);
            view.FocusOn(view.ChampionId.Value);
            foreach (var a in System.Environment.GetCommandLineArgs())
                if (a.StartsWith("--zoom=") && float.TryParse(a.Substring(7), System.Globalization.NumberStyles.Float, System.Globalization.CultureInfo.InvariantCulture, out var z))
                    view.Zoom = z;
        }
        foreach (var a in System.Environment.GetCommandLineArgs())
            if (a.StartsWith("--walk=") && view != null)
                foreach (char c in a.Substring(7).ToUpperInvariant())
                {
                    view.PadWalk = c switch { 'R' => Vector2Int.right, 'L' => Vector2Int.left, 'U' => Vector2Int.down, _ => Vector2Int.up };
                    yield return new WaitForSecondsRealtime(0.15f);
                    view.PadWalk = Vector2Int.zero;
                    yield return new WaitForSecondsRealtime(0.25f);
                }
        foreach (var a in System.Environment.GetCommandLineArgs())
        {
            if (a.StartsWith("--open=")) GetComponent<SimHud>()?.OpenByName(a.Substring(7));
            if (a == "--select-near") GetComponent<SimHud>()?.OpenByName("vicino");
        }
        foreach (var a in System.Environment.GetCommandLineArgs())
            if (a.StartsWith("--map=") && view != null)
            {
                int i = view.LayerIndex(a.Substring(6));
                if (i >= 0)
                {
                    view.FollowCamera = false;
                    view.SetLayer(i);
                }
            }
        yield return new WaitForSecondsRealtime(Mathf.Max(1f, after - (Time.realtimeSinceStartup - t0)));
        yield return new WaitForEndOfFrame();
        ScreenCapture.CaptureScreenshot(path);
        yield return new WaitForSecondsRealtime(1f);
        Application.Quit();
    }
}
