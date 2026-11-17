"""Integration checks for the single-file native launcher using tiny local packages."""
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest
import zlib

from package_single_exe import MAGIC, ROOT

LAUNCHER = ROOT / 'scripts/onefile-launcher/target/x86_64-pc-windows-msvc/release/tsb-onefile.exe'
QA = (ROOT / 'artifacts/single-exe/launcher-tests').resolve()
assert QA.is_relative_to(ROOT.resolve())
QA.mkdir(parents=True, exist_ok=True)


class LauncherChecks(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(dir=QA)
        self.addCleanup(self.temp.cleanup)
        self.folder = Path(self.temp.name)
        self.exe = self.folder / '한글 space.exe'
        self.cache = self.folder / 'cache'
        self.env = dict(os.environ, TSB_CACHE_DIR=str(self.cache))

    def package(self, files=None):
        files = files or {'hello.txt': b'hello', 'data/resources.registry': b'{}'}
        manifest = [{'path': name, 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}
                    for name, data in files.items()]
        encoded = json.dumps(manifest).encode()
        payload = zlib.compress(struct.pack('<I', len(encoded)) + encoded + b''.join(files.values()))
        digest = hashlib.sha256(payload).digest()
        self.exe.write_bytes(LAUNCHER.read_bytes() + payload + MAGIC + struct.pack('<Q', len(payload)) + digest)
        return self.cache / digest.hex()

    def run_exe(self):
        return subprocess.run([str(self.exe), '--verify-only'], cwd=self.folder, env=self.env,
                              capture_output=True, timeout=30, creationflags=subprocess.CREATE_NO_WINDOW)

    def test_extract_reuse_and_mutable_registry(self):
        cache = self.package()
        self.assertEqual(self.run_exe().returncode, 0)
        self.assertEqual((cache / 'hello.txt').read_bytes(), b'hello')
        original = (cache / 'hello.txt').stat().st_mtime_ns
        (cache / 'data/resources.registry').write_bytes(b'{"runtime":"changed"}')
        self.assertEqual(self.run_exe().returncode, 0)
        self.assertEqual((cache / 'hello.txt').stat().st_mtime_ns, original)
        self.assertEqual(len(list(self.cache.iterdir())), 1)

    def test_corrupt_cache_repairs_then_reuses(self):
        cache = self.package()
        self.assertEqual(self.run_exe().returncode, 0)
        (cache / 'hello.txt').write_bytes(b'other')
        self.assertEqual(self.run_exe().returncode, 0)
        self.assertEqual(self.run_exe().returncode, 0)
        copies = list(self.cache.iterdir())
        self.assertEqual(len(copies), 2)
        repaired = next(path for path in copies if path != cache)
        self.assertEqual((repaired / 'hello.txt').read_bytes(), b'hello')

    def test_payload_corruption_is_rejected_before_extraction(self):
        self.package()
        data = bytearray(self.exe.read_bytes())
        data[len(LAUNCHER.read_bytes()) + 8] ^= 1
        self.exe.write_bytes(data)
        self.assertNotEqual(self.run_exe().returncode, 0)
        self.assertFalse(self.cache.exists())

    def test_traversal_is_rejected(self):
        self.package({'../escape.txt': b'unsafe'})
        self.assertNotEqual(self.run_exe().returncode, 0)
        self.assertFalse(self.cache.exists())
        self.assertFalse((self.folder / 'escape.txt').exists())

    def test_overflowing_footer_is_rejected(self):
        self.exe.write_bytes(LAUNCHER.read_bytes() + MAGIC + struct.pack('<Q', 2**64 - 1) + bytes(32))
        self.assertNotEqual(self.run_exe().returncode, 0)
        self.assertFalse(self.cache.exists())

    def test_simultaneous_extraction_is_complete(self):
        self.package()
        processes = [subprocess.Popen([str(self.exe), '--verify-only'], cwd=self.folder, env=self.env,
                                      stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                      creationflags=subprocess.CREATE_NO_WINDOW) for _ in range(2)]
        for process in processes:
            process.communicate(timeout=30)
            self.assertEqual(process.returncode, 0)
        for cache in self.cache.iterdir():
            self.assertTrue((cache / '.ready').is_file())
            self.assertEqual((cache / 'hello.txt').read_bytes(), b'hello')


if __name__ == '__main__':
    unittest.main(verbosity=2)
