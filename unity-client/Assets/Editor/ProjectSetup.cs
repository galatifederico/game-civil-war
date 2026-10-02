using UnityEditor;
using UnityEditor.Build;
using UnityEditor.SceneManagement;
using UnityEngine;

/// <summary>
/// Editor helpers: creates the Main scene and builds a Linux player (also usable from the command line
/// with -executeMethod ProjectSetup.CreateMainScene / ProjectSetup.BuildLinux / ProjectSetup.BuildAndroid).
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

    /// <summary>
    /// Android APK for phones: 64-bit ARM (IL2CPP), landscape, plain HTTP allowed (the sim server on
    /// the LAN has no TLS), drawn inside the safe area so notches do not cover the top bar.
    /// </summary>
    [MenuItem("Fidenza/Build Android (APK)")]
    public static void BuildAndroid()
    {
        if (!System.IO.File.Exists(ScenePath)) CreateMainScene();
        var android = NamedBuildTarget.Android;
        PlayerSettings.productName = "Fidenza";
        PlayerSettings.SetApplicationIdentifier(android, "it.fidenza.client");
        PlayerSettings.SetScriptingBackend(android, ScriptingImplementation.IL2CPP);
        // Low stripping and size-optimised IL2CPP code: less for il2cpp and clang to chew on (8 GB machine).
        PlayerSettings.SetManagedStrippingLevel(android, ManagedStrippingLevel.Low);
        PlayerSettings.SetIl2CppCodeGeneration(android, Il2CppCodeGeneration.OptimizeSize);
        PlayerSettings.Android.targetArchitectures = AndroidArchitecture.ARM64;
        PlayerSettings.Android.renderOutsideSafeArea = false;
        PlayerSettings.insecureHttpOption = InsecureHttpOption.AlwaysAllowed;
        PlayerSettings.defaultInterfaceOrientation = UIOrientation.AutoRotation;
        PlayerSettings.allowedAutorotateToLandscapeLeft = true;
        PlayerSettings.allowedAutorotateToLandscapeRight = true;
        PlayerSettings.allowedAutorotateToPortrait = false;
        PlayerSettings.allowedAutorotateToPortraitUpsideDown = false;
        EditorUserBuildSettings.buildAppBundle = false;
        var report = BuildPipeline.BuildPlayer(new BuildPlayerOptions
        {
            scenes = new[] { ScenePath },
            locationPathName = "Builds/Android/FidenzaClient.apk",
            target = BuildTarget.Android,
        });
        Debug.Log("Build Android: " + report.summary.result);
        if (Application.isBatchMode && report.summary.result != UnityEditor.Build.Reporting.BuildResult.Succeeded) EditorApplication.Exit(1);
    }
}
