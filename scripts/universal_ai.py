import os
import sys
import json
import urllib.request
import urllib.error

def get_slots():
    slots = []
    for i in range(1, 10):
        key = (
            os.environ.get(f"AI_MODEL_API_{i}")
            or os.environ.get(f"AI_API_KEY_{i}")
            or os.environ.get(f"API_KEY_{i}")
            or os.environ.get(f"GEMINI_API_KEY_{i}")
            or os.environ.get(f"OPENAI_API_KEY_{i}")
            or os.environ.get(f"GROQ_API_KEY_{i}")
        )
        if key and key.strip():
            model = (
                os.environ.get(f"AI_MODEL_{i}")
                or os.environ.get(f"MODEL_{i}")
                or os.environ.get("AI_MODEL")
                or os.environ.get("AI_MODEL_NAME")
            )
            endpoint = (
                os.environ.get(f"AI_ENDPOINT_{i}")
                or os.environ.get(f"ENDPOINT_{i}")
                or os.environ.get("AI_ENDPOINT")
                or os.environ.get("AI_BASE_URL")
            )
            slots.append({
                "slot": i,
                "key": key.strip(),
                "model": model.strip() if model else None,
                "endpoint": endpoint.strip() if endpoint else None
            })
    return slots

def get_fallback_keys():
    keys = []
    for var in [
        "AI_API_KEY",
        "AI_API_KEYS",
        "GEMINI_API_KEY",
        "GEMINI_API_KEYS",
        "OPENAI_API_KEY",
        "GROQ_API_KEY",
        "DEEPSEEK_API_KEY",
        "OPENROUTER_API_KEY",
        "MISTRAL_API_KEY",
        "NVIDIA_API_KEY"
    ]:
        val = os.environ.get(var, "").strip()
        if val:
            for k in val.replace(";", ",").replace("\n", ",").split(","):
                k = k.strip()
                if k and k not in keys:
                    keys.append(k)
    return keys

def get_models_for_slot(slot_model, key):
    raw = slot_model or os.environ.get("AI_MODEL") or os.environ.get("AI_MODEL_NAME")
    if raw and raw.strip():
        return [m.strip() for m in raw.replace(";", ",").replace("\n", ",").split(",") if m.strip()]
    if key.startswith("AIza"):
        return ["gemini-2.5-flash", "gemini-2.0-flash", "gemini-1.5-flash", "gemini-3.7-flash"]
    elif key.startswith("gsk_"):
        return ["llama-3.3-70b-versatile", "deepseek-r1-distill-llama-70b", "llama-3.1-8b-instant"]
    elif key.startswith("sk-or-"):
        return ["google/gemini-2.0-flash-exp:free", "meta-llama/llama-3.3-70b-instruct:free", "deepseek/deepseek-chat:free"]
    elif key.startswith("nvapi-"):
        return ["meta/llama-3.3-70b-instruct", "deepseek-ai/deepseek-r1"]
    return ["gemini-2.5-flash", "gpt-4o-mini", "deepseek-chat"]

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

def try_key(key, slot_model, custom_endpoint, prompt):
    endpoint = detect_endpoint(key, custom_endpoint)
    models = get_models_for_slot(slot_model, key)
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

def generate(prompt):
    slots = get_slots()
    for s in slots:
        res = try_key(s["key"], s["model"], s["endpoint"], prompt)
        if res:
            return res
            
    fallback_keys = get_fallback_keys()
    for k in fallback_keys:
        res = try_key(k, None, None, prompt)
        if res:
            return res
            
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
