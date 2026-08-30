import os
import sys
import json
import urllib.request
import urllib.error

def get_keys():
    keys = []
    for var in ["AI_API_KEY", "AI_API_KEYS", "GEMINI_API_KEY", "OPENAI_API_KEY", "GROQ_API_KEY", "DEEPSEEK_API_KEY", "OPENROUTER_API_KEY", "MISTRAL_API_KEY", "NVIDIA_API_KEY"]:
        val = os.environ.get(var, "").strip()
        if val:
            for k in val.replace(";", ",").replace("\n", ",").split(","):
                k = k.strip()
                if k and k not in keys:
                    keys.append(k)
    return keys

def get_models():
    val = os.environ.get("AI_MODEL") or os.environ.get("AI_MODEL_NAME") or os.environ.get("GEMINI_MODEL") or "gemini-2.5-flash"
    models = []
    for m in val.replace(";", ",").replace("\n", ",").split(","):
        m = m.strip()
        if m and m not in models:
            models.append(m)
    return models

def call_gemini(key, model, prompt):
    url = f"https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={key}"
    payload = json.dumps({"contents": [{"parts": [{"text": prompt}]}]}).encode("utf-8")
    req = urllib.request.Request(url, data=payload, headers={"Content-Type": "application/json"}, method="POST")
    with urllib.request.urlopen(req, timeout=30) as resp:
        data = json.loads(resp.read().decode("utf-8"))
        candidates = data.get("candidates", [])
        if candidates:
            parts = candidates[0].get("content", {}).get("parts", [])
            if parts:
                return parts[0].get("text", "").strip()
    return None

def call_openai_compat(endpoint, key, model, prompt):
    if not endpoint.endswith("/chat/completions"):
        endpoint = endpoint.rstrip("/") + "/chat/completions"
    payload = json.dumps({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "temperature": 0.3
    }).encode("utf-8")
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {key}"
    }
    req = urllib.request.Request(endpoint, data=payload, headers=headers, method="POST")
    with urllib.request.urlopen(req, timeout=30) as resp:
        data = json.loads(resp.read().decode("utf-8"))
        choices = data.get("choices", [])
        if choices:
            return choices[0].get("message", {}).get("content", "").strip()
    return None

def detect_endpoint(key, custom_endpoint):
    if custom_endpoint:
        return custom_endpoint
    if key.startswith("gsk_"):
        return "https://api.groq.com/openai/v1"
    elif key.startswith("sk-or-"):
        return "https://openrouter.ai/api/v1"
    elif key.startswith("nvapi-"):
        return "https://integrate.api.nvidia.com/v1"
    elif key.startswith("sk-"):
        return "https://api.openai.com/v1"
    return None

def generate(prompt):
    keys = get_keys()
    if not keys:
        return None
    models = get_models()
    custom_endpoint = os.environ.get("AI_ENDPOINT") or os.environ.get("AI_BASE_URL")
    
    for key in keys:
        endpoint = detect_endpoint(key, custom_endpoint)
        for model in models:
            try:
                if not endpoint and (key.startswith("AIza") or "gemini" in model.lower()):
                    res = call_gemini(key, model, prompt)
                else:
                    ep = endpoint or "https://api.openai.com/v1"
                    res = call_openai_compat(ep, key, model, prompt)
                if res:
                    return res
            except Exception:
                continue
    return None

def main():
    if len(sys.argv) > 1 and sys.argv[1] == "--prompt":
        prompt = " ".join(sys.argv[2:])
    elif not sys.stdin.isatty():
        prompt = sys.stdin.read()
    else:
        prompt = " ".join(sys.argv[1:]) if len(sys.argv) > 1 else ""
    
    if not prompt.strip():
        sys.exit(1)
    
    res = generate(prompt.strip())
    if res:
        print(res)
        sys.exit(0)
    else:
        sys.exit(2)

if __name__ == "__main__":
    main()
