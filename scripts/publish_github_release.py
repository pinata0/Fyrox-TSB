"""Publish the reviewed v0.1 single EXE using the configured Git credential helper.

Credentials remain in memory. Prepare creates a draft; publish exposes only a
draft whose executable, checksum, source commit and notes match local artifacts.
Hotfix replaces a published release's assets after staging and verifying both
new uploads. The existing version tag and release title remain unchanged.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import urllib.error
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
REPO = "pinata0/Fyrox-TSB"
TAG = "v0.1"
ASSET = ROOT / "release/v0.1/TSB.exe"
NOTES = ROOT / "docs/releases/v0.1.md"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["prepare", "publish", "inspect", "hotfix"])
    args = parser.parse_args()
    credential = subprocess.run(
        ["git", "-c", "credential.interactive=never", "credential", "fill"],
        input=f"protocol=https\nhost=github.com\npath={REPO}.git\n\n",
        capture_output=True, text=True, cwd=ROOT,
        env={**os.environ, "GIT_TERMINAL_PROMPT": "0", "GCM_INTERACTIVE": "Never"},
    )
    fields = dict(line.split("=", 1) for line in credential.stdout.splitlines() if "=" in line)
    token = fields.get("password")
    if credential.returncode or not token:
        raise SystemExit("Existing GitHub credential unavailable; sensitive output withheld.")

    def api(path, payload=None, method=None, file=None):
        url = path if path.startswith("https://uploads.github.com/") else f"https://api.github.com/repos/{REPO}" + (f"/{path}" if path else "")
        headers = {"Authorization": f"Bearer {token}", "Accept": "application/vnd.github+json",
                   "User-Agent": "TSB-v0.1-release", "X-GitHub-Api-Version": "2022-11-28"}
        body = None
        if file:
            headers.update({"Content-Type": "application/octet-stream", "Content-Length": str(file.stat().st_size)})
            body = file.open("rb")
        elif payload is not None:
            headers["Content-Type"] = "application/json"
            body = json.dumps(payload).encode("utf-8")
        try:
            request = urllib.request.Request(url, data=body, method=method, headers=headers)
            with urllib.request.urlopen(request, timeout=600 if file else 60) as response:
                raw = response.read()
                return json.loads(raw) if raw else {}
        except urllib.error.HTTPError as error:
            raise SystemExit(f"GitHub {method or 'GET'} failed with HTTP {error.code}; inspect state before retrying.") from None
        finally:
            if file and body:
                body.close()

    sha = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    releases = api("releases?per_page=100")
    release = next((item for item in releases if item["tag_name"] == TAG), None)
    if args.action == "inspect":
        print(json.dumps(None if release is None else {
            **{key: release[key] for key in ["id", "html_url", "draft", "prerelease", "tag_name", "target_commitish"]},
            "assets": [{key: item.get(key) for key in ["id", "name", "size", "digest", "browser_download_url", "state"]} for item in release["assets"]],
        }))
        return
    if api("git/ref/heads/main")["object"]["sha"] != sha:
        raise SystemExit("Push the reviewed local source snapshot to main before creating the release.")
    if not ASSET.is_file():
        raise SystemExit("Missing release/v0.1/TSB.exe")
    digest = hashlib.file_digest(ASSET.open("rb"), "sha256").hexdigest()
    checksum = ASSET.with_suffix(".exe.sha256")
    if checksum.read_text().strip() != f"{digest}  TSB.exe":
        raise SystemExit("EXE checksum file does not match.")
    notes = NOTES.read_text(encoding="utf-8")
    if digest not in notes:
        raise SystemExit("Release notes do not include the final EXE checksum.")
    if args.action == "hotfix":
        if not release or release["draft"]:
            raise SystemExit("Hotfix requires the existing published v0.1 release.")
        replacements = []
        # Upload under temporary names before touching the working download URL.
        for file in [ASSET, checksum]:
            expected = "sha256:" + hashlib.file_digest(file.open("rb"), "sha256").hexdigest()
            current = next((a for a in release["assets"] if a["name"] == file.name), None)
            if current is None:
                raise SystemExit(f"Canonical asset {file.name} missing; inspect before retrying.")
            if current.get("digest") == expected and current["size"] == file.stat().st_size:
                continue
            temporary = f"pending-{digest[:12]}-{file.name}"
            uploaded = next((a for a in release["assets"] if a["name"] == temporary), None)
            if uploaded is None:
                upload_url = release["upload_url"].split("{", 1)[0] + "?" + urllib.parse.urlencode({"name": temporary})
                print(f"Staging {file.name} ({file.stat().st_size} bytes)", flush=True)
                uploaded = api(upload_url, method="POST", file=file)
            if uploaded.get("digest") != expected or uploaded["size"] != file.stat().st_size or uploaded["state"] != "uploaded":
                raise SystemExit("Staged hotfix failed digest/size/state verification; existing assets retained.")
            replacements.append((file, current, uploaded, expected))
        for file, previous, uploaded, expected in replacements:
            live = api(f"releases/{release['id']}")
            current = next((a for a in live["assets"] if a["name"] == file.name), None)
            if current is None or current["id"] != previous["id"] or current.get("digest") != previous.get("digest"):
                raise SystemExit("Release changed during staging; inspect before continuing.")
            backup_name = f"previous-{previous['id']}-{file.name}"
            api(f"releases/assets/{previous['id']}", {"name": backup_name}, "PATCH")
            try:
                result = api(f"releases/assets/{uploaded['id']}", {"name": file.name}, "PATCH")
                if result.get("digest") != expected or result["name"] != file.name:
                    raise SystemExit("Replacement metadata did not match the verified upload.")
            except BaseException:
                live = api(f"releases/{release['id']}")
                if not any(a["name"] == file.name for a in live["assets"]):
                    api(f"releases/assets/{previous['id']}", {"name": file.name}, "PATCH")
                raise
            print(f"Replaced {file.name}: {expected}", flush=True)
        # Publish the matching checksum/notes before removing the old assets.
        release = api(f"releases/{release['id']}", {"body": notes}, "PATCH")
        for file in [ASSET, checksum]:
            item = next(a for a in release["assets"] if a["name"] == file.name)
            expected = "sha256:" + hashlib.file_digest(file.open("rb"), "sha256").hexdigest()
            if item.get("digest") != expected or item["size"] != file.stat().st_size:
                raise SystemExit("Canonical asset verification failed; previous assets retained.")
        for _, previous, _, _ in replacements:
            api(f"releases/assets/{previous['id']}", method="DELETE")
        print(json.dumps({key: release[key] for key in ["id", "html_url", "draft", "prerelease", "tag_name"]}))
        return
    if not release:
        if args.action != "prepare":
            raise SystemExit("Prepare the draft release first.")
        release = api("releases", {"tag_name": TAG, "target_commitish": sha,
            "name": "Trickcal : Shooting Balddagu v0.1", "body": notes,
            "draft": True, "prerelease": True, "make_latest": "false"}, "POST")
        print(f"Created draft release {release['id']}", flush=True)
    if release["target_commitish"] != sha or release["body"] != notes:
        raise SystemExit("Existing release differs from the reviewed source/notes; left unchanged.")
    for file in [ASSET, checksum]:
        expected = "sha256:" + hashlib.file_digest(file.open("rb"), "sha256").hexdigest()
        asset = next((item for item in release["assets"] if item["name"] == file.name), None)
        if asset is None:
            if args.action != "prepare" or not release["draft"]:
                raise SystemExit("Release asset missing; prepare the draft before publication.")
            upload_url = release["upload_url"].split("{", 1)[0] + "?" + urllib.parse.urlencode({"name": file.name})
            print(f"Uploading {file.name} ({file.stat().st_size} bytes)", flush=True)
            asset = api(upload_url, method="POST", file=file)
        if asset.get("digest") != expected or asset["size"] != file.stat().st_size or asset["state"] != "uploaded":
            raise SystemExit("Remote asset failed digest/size/state verification; release remains a draft.")
        print(f"Verified {file.name}: {expected}", flush=True)
    if args.action == "publish" and release["draft"]:
        release = api(f"releases/{release['id']}", {"draft": False, "prerelease": True, "make_latest": "false"}, "PATCH")
        api("", {"description": "트릭컬의 사도들과 함께하는 6 대 6 거점 점령 FPS 팬게임 | Rust · Fyrox | Windows v0.1",
                 "homepage": "https://www.dotgsince2023.co.kr/projects/trickcal-shooting-balddagu"}, "PATCH")
    print(json.dumps({key: release[key] for key in ["id", "html_url", "draft", "prerelease", "tag_name"]}))


if __name__ == "__main__":
    main()
