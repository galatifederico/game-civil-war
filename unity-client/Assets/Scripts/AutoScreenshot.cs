using System.Collections;
using UnityEngine;

/// <summary>
/// Command line helper for automated checks: <c>--screenshot=path.png [--shot-after=8]</c> saves a
/// screenshot after some seconds and quits.
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
        }
        foreach (var a in System.Environment.GetCommandLineArgs())
            if (a == "--faction-tab") GetComponent<SimHud>()?.ShowFactionTab();
        yield return new WaitForSecondsRealtime(Mathf.Max(1f, after - (Time.realtimeSinceStartup - t0)));
        yield return new WaitForEndOfFrame();
        ScreenCapture.CaptureScreenshot(path);
        yield return new WaitForSecondsRealtime(1f);
        Application.Quit();
    }
}
