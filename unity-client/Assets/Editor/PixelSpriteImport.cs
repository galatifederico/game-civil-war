using UnityEditor;

/// <summary>Pixel-perfect import for the generated chibi sheets in Resources/Sprites.</summary>
public class PixelSpriteImport : AssetPostprocessor
{
    void OnPreprocessTexture()
    {
        if (!assetPath.Replace('\\', '/').Contains("Resources/Sprites/")) return;
        var ti = (TextureImporter)assetImporter;
        ti.textureType = TextureImporterType.Default;
        ti.filterMode = UnityEngine.FilterMode.Point;
        ti.textureCompression = TextureImporterCompression.Uncompressed;
        ti.mipmapEnabled = false;
        ti.alphaIsTransparency = true;
        ti.npotScale = TextureImporterNPOTScale.None;
        ti.wrapMode = UnityEngine.TextureWrapMode.Clamp;
        ti.isReadable = true; // tiles are composed into map textures at runtime
    }
}
