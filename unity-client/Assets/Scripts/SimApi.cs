using System;
using System.Collections;
using System.Text;
using Newtonsoft.Json.Linq;
using UnityEngine;
using UnityEngine.Networking;

/// <summary>
/// Minimal HTTP client for the sim_core API. The client is "dumb": it only reads state and sends
/// commands; every rule lives in the Rust engine.
/// </summary>
public class SimApi
{
    public string BaseUrl;

    public SimApi(string baseUrl)
    {
        BaseUrl = baseUrl.TrimEnd('/');
    }

    public IEnumerator Get(string path, Action<JToken> ok, Action<string> fail = null)
    {
        using var req = UnityWebRequest.Get(BaseUrl + path);
        req.timeout = 5;
        yield return req.SendWebRequest();
        Handle(req, ok, fail);
    }

    public IEnumerator Post(string path, JToken body, Action<JToken> ok = null, Action<string> fail = null)
    {
        using var req = new UnityWebRequest(BaseUrl + path, "POST");
        req.uploadHandler = new UploadHandlerRaw(Encoding.UTF8.GetBytes(body.ToString()));
        req.downloadHandler = new DownloadHandlerBuffer();
        req.SetRequestHeader("Content-Type", "application/json");
        req.timeout = 5;
        yield return req.SendWebRequest();
        Handle(req, ok, fail);
    }

    static void Handle(UnityWebRequest req, Action<JToken> ok, Action<string> fail)
    {
        if (req.result != UnityWebRequest.Result.Success)
        {
            var msg = string.IsNullOrEmpty(req.downloadHandler?.text) ? req.error : req.downloadHandler.text;
            fail?.Invoke(msg);
            return;
        }
        JToken json;
        try
        {
            json = JToken.Parse(req.downloadHandler.text);
        }
        catch (Exception e)
        {
            fail?.Invoke("JSON non valido: " + e.Message);
            return;
        }
        ok?.Invoke(json);
    }
}
