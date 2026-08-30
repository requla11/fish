import argparse
import html
import json
import os
import re
import sys
import time
import urllib.parse
import urllib.request
from pathlib import Path

TARGET_LANGS = {
    "vi": {"google": "vi", "label": "Tiếng Việt"},
    "zh-hans": {"google": "zh-CN", "label": "Simplified Chinese"},
    "zh-hant": {"google": "zh-TW", "label": "Traditional Chinese"},
    "ja": {"google": "ja", "label": "Japanese"},
}

DOCS_DIR = Path("docs")
CACHE_FILE = Path(".cache/translation_cache.json")


def load_cache():
    if CACHE_FILE.exists():
        try:
            with open(CACHE_FILE, "r", encoding="utf-8") as f:
                return json.load(f)
        except Exception:
            return {}
    return {}


def save_cache(cache):
    CACHE_FILE.parent.mkdir(parents=True, exist_ok=True)
    with open(CACHE_FILE, "w", encoding="utf-8") as f:
        json.dump(cache, f, ensure_ascii=False, indent=2)


def translate_with_gemini(text, target_lang, api_key):
    lang_name = TARGET_LANGS.get(target_lang, {}).get("label", target_lang)
    prompt = (
        f"Translate the following text into {lang_name}. "
        f"Preserve all special placeholders like ___TOKEN_0___, markdown symbols, and technical terms. "
        f"Return ONLY the translated text without explanations or markdown formatting blocks:\n\n{text}"
    )
    payload = json.dumps({"contents": [{"parts": [{"text": prompt}]}]}).encode("utf-8")
    for model in ["gemini-3.7-flash", "gemini-2.5-flash", "gemini-2.0-flash", "gemini-1.5-flash"]:
        try:
            url = f"https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={api_key}"
            req = urllib.request.Request(url, data=payload, headers={"Content-Type": "application/json"})
            with urllib.request.urlopen(req, timeout=15) as resp:
                data = json.loads(resp.read().decode("utf-8"))
                res = data["candidates"][0]["content"]["parts"][0]["text"].strip()
                if res:
                    return res
        except Exception:
            continue
    return None


def translate_with_google_api(text, target_lang, api_key):
    google_code = TARGET_LANGS.get(target_lang, {}).get("google", target_lang)
    url = f"https://translation.googleapis.com/language/translate/v2?key={api_key}"
    payload = json.dumps({
        "q": text,
        "source": "en",
        "target": google_code,
        "format": "text"
    }).encode("utf-8")
    req = urllib.request.Request(url, data=payload, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=15) as resp:
        data = json.loads(resp.read().decode("utf-8"))
        return data["data"]["translations"][0]["translatedText"]


def translate_with_google_web(text, target_lang):
    google_code = TARGET_LANGS.get(target_lang, {}).get("google", target_lang)
    encoded = urllib.parse.quote(text)
    url = f"https://translate.google.com/m?sl=en&tl={google_code}&q={encoded}"
    req = urllib.request.Request(
        url,
        headers={
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36"
        }
    )
    with urllib.request.urlopen(req, timeout=15) as resp:
        content = resp.read().decode("utf-8")
        match = re.search(r'class="result-container">([^<]+)', content)
        if match:
            return html.unescape(match.group(1)).strip()
    return None


def translate_with_mymemory(text, target_lang):
    google_code = TARGET_LANGS.get(target_lang, {}).get("google", target_lang)
    encoded = urllib.parse.quote(text)
    url = f"https://api.mymemory.translated.net/get?q={encoded}&langpair=en|{google_code}"
    req = urllib.request.Request(
        url,
        headers={"User-Agent": "Mozilla/5.0"}
    )
    with urllib.request.urlopen(req, timeout=15) as resp:
        data = json.loads(resp.read().decode("utf-8"))
        if data.get("responseStatus") == 200:
            return data["responseData"]["translatedText"]
    return None


def translate_single_text(text, target_lang, cache, force=False):
    if not text or not text.strip():
        return text

    cache_key = f"{target_lang}:{text.strip()}"
    if not force and cache_key in cache:
        return cache[cache_key]

    gemini_key = os.environ.get("GEMINI_API_KEY")
    google_key = os.environ.get("GOOGLE_TRANSLATE_API_KEY")

    result = None
    last_err = None

    if gemini_key:
        try:
            result = translate_with_gemini(text, target_lang, gemini_key)
        except Exception as e:
            last_err = e

    if not result and google_key:
        try:
            result = translate_with_google_api(text, target_lang, google_key)
        except Exception as e:
            last_err = e

    if not result:
        for _ in range(3):
            try:
                result = translate_with_google_web(text, target_lang)
                if result:
                    break
            except Exception as e:
                last_err = e
                time.sleep(1)

    if not result:
        try:
            result = translate_with_mymemory(text, target_lang)
        except Exception as e:
            last_err = e

    if result:
        cache[cache_key] = result
        return result

    return text


class MarkdownProtector:
    def __init__(self):
        self.tokens = []

    def add_token(self, content):
        idx = len(self.tokens)
        token = f"___FTO_{idx}___"
        self.tokens.append((token, content))
        return token

    def protect(self, text):
        def repl_fenced(match):
            return self.add_token(match.group(0))

        text = re.sub(r'```[\s\S]*?```', repl_fenced, text)

        def repl_html_comment(match):
            return self.add_token(match.group(0))

        text = re.sub(r'<!--[\s\S]*?-->', repl_html_comment, text)

        def repl_inline_code(match):
            return self.add_token(match.group(0))

        text = re.sub(r'`[^`\n]+`', repl_inline_code, text)

        def repl_badge(match):
            return self.add_token(match.group(0))

        text = re.sub(r'\[!\[[^\]]*\]\([^)]+\)\]\([^)]+\)', repl_badge, text)

        def repl_image(match):
            return self.add_token(match.group(0))

        text = re.sub(r'!\[[^\]]*\]\([^)]+\)', repl_image, text)

        def repl_link_url(match):
            link_text = match.group(1)
            link_url = match.group(2)
            url_token = self.add_token(link_url)
            return f"[{link_text}]({url_token})"

        text = re.sub(r'\[([^\]]+)\]\(([^)]+)\)', repl_link_url, text)

        def repl_html_tags(match):
            return self.add_token(match.group(0))

        text = re.sub(r'<[^>]+>', repl_html_tags, text)

        return text

    def unprotect(self, text):
        for token, original in reversed(self.tokens):
            text = text.replace(token, original)
        return text


def translate_markdown_line(line, target_lang, cache, force=False):
    stripped = line.strip()
    if not stripped:
        return line

    if re.match(r'^[-*_]{3,}$', stripped):
        return line

    if re.match(r'^\|?\s*[-:]+[-| :]*\|?\s*$', stripped):
        return line

    if stripped.startswith("|") and stripped.endswith("|"):
        cells = [c.strip() for c in stripped.split("|")[1:-1]]
        translated_cells = []
        for cell in cells:
            if not cell or re.match(r'^[-:]+$', cell):
                translated_cells.append(cell)
            else:
                prot = MarkdownProtector()
                p_cell = prot.protect(cell)
                t_cell = translate_single_text(p_cell, target_lang, cache, force)
                u_cell = prot.unprotect(t_cell)
                translated_cells.append(u_cell)
        return "| " + " | ".join(translated_cells) + " |"

    heading_match = re.match(r'^(#{1,6}\s+)(.*)$', line)
    if heading_match:
        prefix, content = heading_match.groups()
        prot = MarkdownProtector()
        p_content = prot.protect(content)
        t_content = translate_single_text(p_content, target_lang, cache, force)
        u_content = prot.unprotect(t_content)
        return f"{prefix}{u_content}"

    quote_match = re.match(r'^(>\s*(?:\[!(?:NOTE|TIP|IMPORTANT|WARNING|CAUTION)\])?\s*)(.*)$', line)
    if quote_match:
        prefix, content = quote_match.groups()
        if content:
            prot = MarkdownProtector()
            p_content = prot.protect(content)
            t_content = translate_single_text(p_content, target_lang, cache, force)
            u_content = prot.unprotect(t_content)
            return f"{prefix}{u_content}"
        return line

    list_match = re.match(r'^(\s*[-*+]\s+|\s*\d+\.\s+)(.*)$', line)
    if list_match:
        prefix, content = list_match.groups()
        prot = MarkdownProtector()
        p_content = prot.protect(content)
        t_content = translate_single_text(p_content, target_lang, cache, force)
        u_content = prot.unprotect(t_content)
        return f"{prefix}{u_content}"

    prot = MarkdownProtector()
    p_line = prot.protect(line)
    t_line = translate_single_text(p_line, target_lang, cache, force)
    u_line = prot.unprotect(t_line)
    return u_line


def translate_markdown_file(src_path, dest_path, target_lang, cache, force=False):
    with open(src_path, "r", encoding="utf-8") as f:
        content = f.read()

    lines = content.splitlines()
    in_code_block = False
    in_frontmatter = False
    translated_lines = []

    for idx, line in enumerate(lines):
        if idx == 0 and line.strip() == "---":
            in_frontmatter = True
            translated_lines.append(line)
            continue

        if in_frontmatter:
            translated_lines.append(line)
            if line.strip() == "---":
                in_frontmatter = False
            continue

        if line.strip().startswith("```"):
            in_code_block = not in_code_block
            translated_lines.append(line)
            continue

        if in_code_block:
            translated_lines.append(line)
            continue

        trans_line = translate_markdown_line(line, target_lang, cache, force)
        translated_lines.append(trans_line)

    output_text = "\n".join(translated_lines)
    if not output_text.endswith("\n"):
        output_text += "\n"

    dest_path.parent.mkdir(parents=True, exist_ok=True)
    with open(dest_path, "w", encoding="utf-8") as f:
        f.write(output_text)


def collect_canonical_docs():
    excluded = set(list(TARGET_LANGS.keys()) + ["public"])
    canonical = []
    for p in DOCS_DIR.rglob("*.md"):
        rel = p.relative_to(DOCS_DIR)
        if rel.parts[0] not in excluded:
            canonical.append(rel)
    return sorted(canonical)


def check_docs():
    canonical = collect_canonical_docs()
    all_ok = True
    print(f"Checking {len(canonical)} documentation files across {len(TARGET_LANGS)} languages...")
    for rel in canonical:
        for lang in TARGET_LANGS:
            dest = DOCS_DIR / lang / rel
            if not dest.exists():
                print(f"[MISSING] {lang}/{rel}")
                all_ok = False
    if all_ok:
        print("All language documentation files are in sync.")
    return 0 if all_ok else 1


def translate_all(target_langs, force=False):
    canonical = collect_canonical_docs()
    cache = load_cache()
    total = len(canonical) * len(target_langs)
    done = 0

    print(f"Translating {len(canonical)} docs into languages: {', '.join(target_langs)}")

    for rel in canonical:
        src = DOCS_DIR / rel
        for lang in target_langs:
            dest = DOCS_DIR / lang / rel
            done += 1
            print(f"[{done}/{total}] Translating {rel} -> {lang}...")
            try:
                translate_markdown_file(src, dest, lang, cache, force)
            except Exception as e:
                print(f"Error translating {rel} to {lang}: {e}")
            if done % 10 == 0:
                save_cache(cache)

    save_cache(cache)
    print("Documentation translation complete.")
    return 0


def main():
    parser = argparse.ArgumentParser(description="Automated Markdown Documentation Translator")
    parser.add_argument("--all", action="store_true", help="Translate all documentation files")
    parser.add_argument("--check", action="store_true", help="Check for missing translations")
    parser.add_argument("--file", type=str, help="Specific markdown file to translate")
    parser.add_argument("--lang", type=str, help="Comma-separated language codes (vi, zh-hans, zh-hant, ja)")
    parser.add_argument("--force", action="store_true", help="Force re-translation bypassing cache")

    args = parser.parse_args()

    selected_langs = list(TARGET_LANGS.keys())
    if args.lang:
        selected_langs = [l.strip() for l in args.lang.split(",") if l.strip() in TARGET_LANGS]

    if args.check:
        sys.exit(check_docs())

    if args.file:
        src_path = Path(args.file)
        if not src_path.exists():
            print(f"File not found: {src_path}")
            sys.exit(1)
        rel = src_path.relative_to(DOCS_DIR) if src_path.is_relative_to(DOCS_DIR) else src_path.name
        cache = load_cache()
        for lang in selected_langs:
            dest = DOCS_DIR / lang / rel
            print(f"Translating {src_path} -> {dest} ({lang})...")
            translate_markdown_file(src_path, dest, lang, cache, args.force)
        save_cache(cache)
        sys.exit(0)

    if args.all or len(sys.argv) == 1:
        sys.exit(translate_all(selected_langs, args.force))


if __name__ == "__main__":
    main()
