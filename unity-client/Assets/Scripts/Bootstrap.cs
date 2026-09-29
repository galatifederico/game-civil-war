using UnityEngine;

/// <summary>
/// Creates the client in any scene when Play is pressed, so no scene setup is required.
/// </summary>
public static class Bootstrap
{
    [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.AfterSceneLoad)]
    static void Init()
    {
        Application.runInBackground = true;
        var view = Object.FindFirstObjectByType<SimView>();
        if (view == null)
        {
            var go = new GameObject("SimClient");
            view = go.AddComponent<SimView>();
            go.AddComponent<SimHud>();
        }
        AutoScreenshot.InstallFromArgs(view.gameObject);
    }
}
