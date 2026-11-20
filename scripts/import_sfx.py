"""Import the curated, unmodified Kenney CC0 clips from downloaded source zips.

Download interface-sounds, impact-sounds, rpg-audio and sci-fi-sounds from
https://kenney.nl/assets into artifacts/sound-sources/<pack>.zip first.
"""
import hashlib
import json
from pathlib import Path
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
PACKS = {
    "interface-sounds": {
        "click": "click_001", "confirm": "confirmation_002", "back": "back_001",
        "pluck": "pluck_001", "heal": "glass_001", "ready": "confirmation_004",
        "capture": "confirmation_003", "win": "maximize_004", "lose": "error_003",
        "death": "minimize_006", "respawn": "maximize_006",
    },
    "impact-sounds": {
        "hit": "impactPunch_medium_000", "hurt": "impactSoft_heavy_000",
        "wall": "impactGeneric_light_002", "blunt": "impactPunch_heavy_000",
        "shield": "impactGlass_medium_000", "step0": "footstep_concrete_000",
        "step1": "footstep_concrete_001", "land": "footstep_concrete_003",
    },
    "rpg-audio": {
        "slash": "knifeSlice", "dart": "knifeSlice2", "reload": "beltHandle1",
        "reload_done": "metalLatch", "dash": "cloth1", "jump": "cloth2",
    },
    "sci-fi-sounds": {
        "water": "slime_000", "magic": "laserSmall_000", "laser": "laserLarge_000",
        "rifle": "laserSmall_003", "skill": "forceField_000",
        "ultimate": "explosionCrunch_001", "portal": "forceField_004",
    },
}


def main():
    out = ROOT / "assets/audio/sfx"
    out.mkdir(parents=True, exist_ok=True)
    manifest = []
    for pack, clips in PACKS.items():
        with ZipFile(ROOT / "artifacts/sound-sources" / (pack + ".zip")) as archive:
            (out / ("LICENSE-" + pack + ".txt")).write_bytes(archive.read("License.txt"))
            for name, original in clips.items():
                source = "Audio/" + original + ".ogg"
                data = archive.read(source)
                assert data.startswith(b"OggS"), source
                (out / (name + ".ogg")).write_bytes(data)
                manifest.append({"file": name + ".ogg", "author": "Kenney",
                                 "license": "CC0-1.0", "source": "https://kenney.nl/assets/" + pack,
                                 "original": source, "sha256": hashlib.sha256(data).hexdigest()})
    (out / "sources.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(f"Imported {len(manifest)} OGG clips with source licenses and SHA-256 hashes.")


if __name__ == "__main__":
    main()
