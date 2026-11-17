"""Collect observed public reference URLs; reference files are not runtime assets."""
import concurrent.futures
import hashlib
import json
from pathlib import Path
import re
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "assets/references"
ITEMS = json.loads((ROOT / "artifacts/reference-research/spines-index.json").read_text(encoding="utf-8"))
NAMES = [
    ("vivi", "비비", "vivi"), ("naia", "나이아", "naia"),
    ("erpin", "에르핀", "erpin"), ("id", "이드", "ed"),
    ("aya", "아야", "aya"), ("picora", "피코라", "picora"),
    ("chloe", "클로에", "chloe"), ("renua", "리뉴아", "renewaawaken"),
    ("mayo", "마요", "mayo"), ("bella", "벨라", "vela"),
    ("ui", "우이", "ui"), ("butter", "버터", "butter"),
    ("komi", "코미", "kommy"), ("hilde", "힐데", "hilde"),
    ("shion", "시온 더 다크불릿", "xxionx"),
]

def get(url):
    request = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
    return urllib.request.urlopen(request, timeout=40).read()

def fetch(entry):
    index, (slug, name, source_id) = entry
    hero = next(i for i in ITEMS if i["id"] == source_id)
    standing = next((i for i in ITEMS if i["title"] == source_id and i["subtitle"] == "standing"), hero)
    record = {"id": index, "slug": slug, "name": name, "retrieved": "2026-11-12", "publisher": "EPID Games", "host": "Cheeky community reference viewer", "files": [], "motion_viewer": "https://wiki.cheeky.tsuncloud.com" + hero["href"], "standing_viewer": "https://wiki.cheeky.tsuncloud.com" + standing["href"]}
    for kind, item in [("battle", hero), ("standing", standing)]:
        url = item["iconUrl"]
        data = get(url)
        target = OUT / (slug + "-" + kind + ".webp")
        target.write_bytes(data)
        record["files"].append({"path": target.relative_to(ROOT).as_posix(), "url": url, "sha256": hashlib.sha256(data).hexdigest()})
    html = get(record["motion_viewer"]).decode("utf-8")
    stream = "\n".join(json.loads(m)[1] for m in re.findall(r'self.__next_f.push\((\[1,.*?\])\)</script>', html))
    spine = re.search(r'"spine":(\{"skeleton":.*?\})', stream)
    if spine:
        record["spine_sources"] = json.loads(spine.group(1))
        raw = get(record["spine_sources"]["skeleton"])
        # Keep a local motion-reference file, separate from our newly authored rig.
        (OUT / (slug + "-motion-reference.skel")).write_bytes(raw)
        record["motion_strings"] = sorted(set(s.decode() for s in re.findall(rb"[A-Za-z][A-Za-z_0-9/ -]{3,48}", raw) if re.search(rb"(?i)(walk|run|idle|attack|skill|victory|dance|move|win|die|dead)",s)))
    print(slug, "reference OK", flush=True)
    return record

if __name__ == "__main__":
    OUT.mkdir(parents=True, exist_ok=True)
    with concurrent.futures.ThreadPoolExecutor(max_workers=5) as pool:
        records = list(pool.map(fetch, enumerate(NAMES)))
    (OUT / "manifest.json").write_text(json.dumps(records, ensure_ascii=False, indent=2), encoding="utf-8")
