"""Auto-fill packaging manifests from the latest GitHub Release.

Fetches release assets + SHA256 digests from the GitHub API and patches
packaging/fish.json (Scoop), packaging/fish.rb (Homebrew), and
packaging/fish.winget.yaml (Winget) with real URLs and hashes.

Usage:
    python scripts/update_packaging.py [--tag v0.5.0] [--repo owner/name]

No review/approval needed: Scoop installs via URL, Homebrew uses a
personal tap — both consume these files directly.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
import tempfile
import urllib.request
from pathlib import Path

PACKAGING_DIR = Path(__file__).resolve().parent.parent / "packaging"


def gh_api(path: str) -> dict:
    out = subprocess.run(
        ["gh", "api", path, "--jq", "."],
        capture_output=True,
        text=True,
        check=True,
    )
    return json.loads(out.stdout)


def resolve_tag(repo: str, tag: str | None) -> str:
    if tag:
        return tag.lstrip("v")
    data = gh_api(f"repos/{repo}/releases/latest")
    return data["tag_name"].lstrip("v")


def download_and_hash(repo: str, tag: str, asset_name: str) -> str:
    url = f"https://github.com/{repo}/releases/download/v{tag}/{asset_name}"
    with tempfile.TemporaryDirectory() as tmp:
        dest = Path(tmp) / asset_name
        urllib.request.urlretrieve(url, dest)
        return hashlib.sha256(dest.read_bytes()).hexdigest()


def patch_scoop(tag: str, sha_windows: str) -> None:
    path = PACKAGING_DIR / "fish.json"
    text = path.read_text(encoding="utf-8")
    text = re.sub(r'"version":\s*"[^"]+"', f'"version": "{tag}"', text)
    text = re.sub(r"v\d+\.\d+\.\d+/fish-[\w.-]+\.zip", f"v{tag}/fish-{tag}-x86_64-pc-windows-msvc.zip", text)
    text = re.sub(r'"hash":\s*"REPLACE_WITH_SHA256_AT_RELEASE"', f'"hash": "{sha_windows}"', text)
    path.write_text(text, encoding="utf-8")
    print(f"[scoop] patched {path.name} -> v{tag}")


def patch_homebrew(tag: str, sha_source: str) -> None:
    path = PACKAGING_DIR / "fish.rb"
    text = path.read_text(encoding="utf-8")
    text = re.sub(r"/archive/refs/tags/v[\w.]+\.tar\.gz", f"/archive/refs/tags/v{tag}.tar.gz", text)
    text = re.sub(r'sha256 "[^"]*"', f'sha256 "{sha_source}"', text)
    text = re.sub(r'version\.to_s', 'version.to_s', text)
    path.write_text(text, encoding="utf-8")
    print(f"[brew ] patched {path.name} -> v{tag}")


def patch_winget(tag: str, sha_windows: str) -> None:
    path = PACKAGING_DIR / "fish.winget.yaml"
    text = path.read_text(encoding="utf-8")
    text = re.sub(r"PackageVersion:\s*[\w.]+", f"PackageVersion: {tag}", text)
    text = re.sub(r"v\d+\.\d+\.\d+/fish-[\w.-]+\.zip", f"v{tag}/fish-{tag}-x86_64-pc-windows-msvc.zip", text)
    text = re.sub(r"InstallerSha256:\s*\S+", f"InstallerSha256: {sha_windows.upper()}", text)
    path.write_text(text, encoding="utf-8")
    print(f"[winget] patched {path.name} -> v{tag}")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--tag", default=None, help="release tag without v prefix; defaults to latest")
    ap.add_argument("--repo", default="requla11/fish")
    args = ap.parse_args()

    tag = resolve_tag(args.repo, args.tag)
    print(f"target release: v{tag}")

    win_asset = f"fish-{tag}-x86_64-pc-windows-msvc.zip"
    src_url = f"https://github.com/{args.repo}/archive/refs/tags/v{tag}.tar.gz"

    with tempfile.TemporaryDirectory() as tmp:
        # Windows zip hash for scoop/winget.
        try:
            sha_win = download_and_hash(args.repo, tag, win_asset)
        except Exception as exc:
            print(f"warn: cannot hash {win_asset}: {exc}", file=sys.stderr)
            sha_win = None

        # Source tarball hash for Homebrew.
        src_dest = Path(tmp) / "src.tar.gz"
        try:
            urllib.request.urlretrieve(src_url, src_dest)
            sha_src = hashlib.sha256(src_dest.read_bytes()).hexdigest()
        except Exception as exc:
            print(f"warn: cannot hash source tarball: {exc}", file=sys.stderr)
            sha_src = None

    if sha_win:
        patch_scoop(tag, sha_win)
        patch_winget(tag, sha_win)
    if sha_src:
        patch_homebrew(tag, sha_src)

    missing = [n for n, h in [("scoop", sha_win), ("homebrew", sha_src)] if not h]
    if missing:
        print(f"incomplete: {', '.join(missing)} need a published release first", file=sys.stderr)
        return 1
    print("all manifests updated.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
