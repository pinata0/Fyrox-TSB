"""Fetch original Trickcal UI markers and losslessly convert their existing alpha to PNG."""
import hashlib
import json
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from urllib.request import Request, urlopen

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "assets/references/trait-markers"
OUTPUT = ROOT / "assets/ui/markers"
PAGE = "https://www.prydwen.gg/trickcal/characters/"
CDN = "https://cdn.prydwen.gg/images/trickcal/icons/"
# Ordered exactly like the existing roster enums, not the English website filters.
MARKERS = [
    ("personality-pure", "순수", "element_innocence"),
    ("personality-cool", "냉정", "element_composed"),
    ("personality-mad", "광기", "element_madness"),
    ("personality-lively", "활발", "element_vivacious"),
    ("personality-gloomy", "우울", "element_depressed"),
    ("race-fairy", "요정", "race_sprites"),
    ("race-beast", "수인", "race_werebeast"),
    ("race-elf", "엘프", "race_elves"),
    ("race-spirit", "정령", "race_elementals"),
    ("race-ghost", "유령", "race_phantoms"),
    ("race-dragon", "용족", "race_dragons"),
    ("race-witch", "마녀", "race_witches"),
    ("position-front", "전열", "position_front"),
    ("position-middle", "중열", "position_middle"),
    ("position-rear", "후열", "position_back"),
]


def fetch(entry):
    slug, label, original = entry
    source = SOURCE / f"{original}.webp"
    url = f"{CDN}{original}.webp"
    if not source.exists():
        request = Request(url, headers={"User-Agent": "Mozilla/5.0"})
        source.write_bytes(urlopen(request, timeout=45).read())
    with Image.open(source) as image:
        rgba = image.convert("RGBA")
        alpha = rgba.getchannel("A")
        # Stop rather than silently shipping an opaque background or redrawing the art.
        assert alpha.getextrema() == (0, 255), f"Missing transparency: {source}"
        # Some WEBP exports retain a 1/255 antialiasing sample at one corner.
        assert all(alpha.getpixel(p) <= 1 for p in (
            (0, 0), (rgba.width - 1, 0), (0, rgba.height - 1),
            (rgba.width - 1, rgba.height - 1))), f"Opaque corners: {source}"
        target = OUTPUT / f"{slug}.png"
        rgba.save(target)
        with Image.open(target) as converted:
            assert converted.tobytes() == rgba.tobytes(), "Conversion changed pixels"
        result = {
            "file": str(target.relative_to(ROOT)).replace("\\", "/"),
            "label": label, "page": PAGE, "image": url,
            "source_file": str(source.relative_to(ROOT)).replace("\\", "/"),
            "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
            "png_sha256": hashlib.sha256(target.read_bytes()).hexdigest(),
            "size": list(rgba.size), "alpha_range": list(alpha.getextrema()),
            "transparent_pixels": alpha.histogram()[0],
            "processing": "Existing original transparency preserved; lossless RGBA PNG conversion; no redraw, crop or recoloring.",
        }
        print(f"MARKER_OK {slug} {rgba.size} alpha={alpha.getextrema()}")
        return result


def main():
    SOURCE.mkdir(parents=True, exist_ok=True)
    OUTPUT.mkdir(parents=True, exist_ok=True)
    with ThreadPoolExecutor(max_workers=5) as pool:
        markers = list(pool.map(fetch, MARKERS))
    (OUTPUT / "sources.json").write_text(json.dumps({
        "accessed": "2026-11-13", "source": "Trickcal game UI markers hosted by Prydwen",
        "rights": "Trickcal artwork belongs to EPID Games; no ownership or license transfer is implied.",
        "markers": markers,
    }, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
