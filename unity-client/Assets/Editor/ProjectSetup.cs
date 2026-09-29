using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;

/// <summary>
/// Editor helpers: creates the Main scene and builds a Linux player (also usable from the command line
/// with -executeMethod ProjectSetup.CreateMainScene / ProjectSetup.BuildLinux).
/// </summary>
public static class ProjectSetup
{
    const string ScenePath = "Assets/Scenes/Main.unity";

    [MenuItem("Fidenza/Crea scena Main")]
    public static void CreateMainScene()
    {
        var scene = EditorSceneManager.NewScene(NewSceneSetup.DefaultGameObjects, NewSceneMode.Single);
        var go = new GameObject("SimClient");
        go.AddComponent<SimView>();
        go.AddComponent<SimHud>();
        EditorSceneManager.SaveScene(scene, ScenePath);
        EditorBuildSettings.scenes = new[] { new EditorBuildSettingsScene(ScenePath, true) };
        Debug.Log("Scena creata: " + ScenePath);
    }

    [MenuItem("Fidenza/Build Linux")]
    public static void BuildLinux()
    {
        if (!System.IO.File.Exists(ScenePath)) CreateMainScene();
        var report = BuildPipeline.BuildPlayer(new BuildPlayerOptions
        {
            scenes = new[] { ScenePath },
            locationPathName = "Builds/Linux/FidenzaClient.x86_64",
            target = BuildTarget.StandaloneLinux64,
        });
        Debug.Log("Build: " + report.summary.result);
    }
}
