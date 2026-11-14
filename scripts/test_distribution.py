"""Distribution architecture guards, without downloads or release builds.

Run: python3 -I -m unittest discover -s scripts -p test_distribution.py
"""

import hashlib
import os
import shutil
import tarfile
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parent.parent


class DistributionTests(unittest.TestCase):
    def setUp(self):
        self.work = tempfile.TemporaryDirectory(prefix="canopy-distribution-")
        self.addCleanup(self.work.cleanup)
        self.bin = Path(self.work.name)
        self.stub("uname", 'case "$1" in -s) echo "$TEST_OS";; -m) echo "$TEST_ARCH";; esac')
        self.stub("curl", 'echo "unexpected download" >&2; exit 97')
        self.stub("cargo", 'echo "unexpected build" >&2; exit 97')
        self.stub("sysctl", 'exit 1')

    def stub(self, name, body):
        path = self.bin / name
        path.write_text("#!/bin/sh\n" + body + "\n")
        path.chmod(0o755)

    def run_script(self, script, os_name, arch, *args):
        env = dict(os.environ, PATH=f"{self.bin}:{os.environ['PATH']}", TEST_OS=os_name, TEST_ARCH=arch, CANOPY_RELEASES_URL="https://releases.example")
        return subprocess.run(
            [str(ROOT / script), *args], cwd=ROOT, env=env,
            capture_output=True, text=True, check=False,
        )

    def test_installer_defaults_to_the_latest_github_release(self):
        log = self.bin / "urls"
        self.stub("curl", f'for a; do case $a in -*) ;; *) echo "$a" >> "{log}" ;; esac; done; exit 22')
        env = dict(os.environ, PATH=f"{self.bin}:{os.environ['PATH']}", TEST_OS="Linux", TEST_ARCH="x86_64")
        env.pop("CANOPY_RELEASES_URL", None)
        result = subprocess.run([str(ROOT / "distribution/install.sh")], env=env,
                                capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 1)
        self.assertEqual(log.read_text().splitlines()[0],
                         "https://github.com/Krishnapthm/canopy/releases/latest/download/SHA256SUMS")

    def test_intel_mac_installer_stops_before_download(self):
        for arch in ("x86_64", "amd64"):
            with self.subTest(arch=arch):
                result = self.run_script("distribution/install.sh", "Darwin", arch)
                self.assertEqual(result.returncode, 1)
                self.assertIn("Intel Macs aren't supported; build from source", result.stderr)
                self.assertNotIn("unexpected download", result.stderr)

    def test_windows_shells_are_pointed_at_the_powershell_installer(self):
        for os_name in ("MINGW64_NT-10.0-26300", "MSYS_NT-10.0-26300", "CYGWIN_NT-10.0-26300"):
            with self.subTest(os_name=os_name):
                result = self.run_script("distribution/install.sh", os_name, "x86_64")
                self.assertEqual(result.returncode, 1)
                self.assertIn("https://releases.example/install.ps1 | iex", result.stderr)
                self.assertNotIn("unexpected download", result.stderr)

    def test_unknown_os_installer_names_it(self):
        result = self.run_script("distribution/install.sh", "FreeBSD", "x86_64")
        self.assertEqual(result.returncode, 1)
        self.assertIn("no build for FreeBSD", result.stderr)
        self.assertNotIn("unexpected download", result.stderr)

    def test_rosetta_mac_installer_uses_arm64(self):
        for key in ("hw.optional.arm64", "sysctl.proc_translated"):
            with self.subTest(key=key):
                self.stub("sysctl", f'[ "$2" = "{key}" ] && echo 1')
                self.stub("curl", '''while [ "$#" -gt 0 ]; do
  if [ "$1" = -o ]; then
    printf '%064d  canopy-macos-aarch64.tar.gz\n' 0 > "$2"
    exit 0
  fi
  shift
done
exit 97''')
                result = self.run_script("distribution/install.sh", "Darwin", "x86_64")
                self.assertEqual(result.returncode, 1)
                self.assertIn("Installing Canopy for macos-aarch64", result.stdout)
                self.assertIn("download is damaged", result.stderr)
                self.assertNotIn("Intel Macs", result.stderr)

    def linux_install(self, core_installer="ok", have_core=False):
        """Runs install.sh for Linux against a stub release and core installer."""
        home = self.bin / "home"
        local_bin = home / ".local/bin"
        local_bin.mkdir(parents=True, exist_ok=True)
        if have_core:
            (local_bin / "herdr-core").touch()
        source = self.bin / "source/canopy"
        source.mkdir(parents=True)
        (source / "canopy").write_text("app")
        (source / "canopy.svg").write_text("<svg/>")
        archive = self.bin / "release.tar.gz"
        with tarfile.open(archive, "w:gz") as tar:
            tar.add(source, arcname="canopy")
        sums = self.bin / "sums"
        sums.write_text(f"{'0' * 64}  canopy-linux-aarch64.tar.gz\n"
                        f"{hashlib.sha256(archive.read_bytes()).hexdigest()}  canopy-linux-x86_64.tar.gz\n")
        core = self.bin / "core.sh"
        core.write_text('echo "core into $HC_INSTALL_DIR"; touch "$HC_INSTALL_DIR/herdr-core"\n')
        log = self.bin / "urls"
        get_core = {"ok": f'cp "{core}" "$out"', "unreachable": "exit 6"}[core_installer]
        self.stub("curl", f'''out=; url=; prev=
for a; do
  if [ "$prev" = -o ]; then out=$a; else case $a in -*) ;; *) url=$a ;; esac; fi
  prev=$a
done
echo "$url" >> "{log}"
case $url in
  https://releases.example/SHA256SUMS) cp "{sums}" "$out" ;;
  https://releases.example/canopy-linux-x86_64.tar.gz) cp "{archive}" "$out" ;;
  https://releases.herdr.dev/core/install.sh) {get_core} ;;
  *) exit 97 ;;
esac''')
        # A bare system PATH: a herdr-core installed on this machine must not count.
        env = dict(PATH=f"{self.bin}:/usr/bin:/bin", TEST_OS="Linux", TEST_ARCH="x86_64", HOME=str(home),
                   XDG_DATA_HOME=str(home / ".local/share"), CANOPY_RELEASES_URL="https://releases.example")
        result = subprocess.run(["sh", str(ROOT / "distribution/install.sh")], env=env,
                                capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual((home / ".local/share/canopy/canopy").read_text(), "app")
        return result, log.read_text().splitlines(), local_bin

    def test_installer_adds_herdr_core_when_missing(self):
        result, urls, local_bin = self.linux_install()
        self.assertIn("https://releases.herdr.dev/core/install.sh", urls)
        self.assertIn(f"core into {local_bin}", result.stdout)
        self.assertTrue((local_bin / "herdr-core").exists())

    def test_installer_keeps_an_existing_herdr_core(self):
        result, urls, _ = self.linux_install(have_core=True)
        self.assertNotIn("https://releases.herdr.dev/core/install.sh", urls)
        self.assertNotIn("herdr-core", result.stdout)

    def test_unreachable_core_installer_still_installs_the_app(self):
        result, _, local_bin = self.linux_install(core_installer="unreachable")
        self.assertIn("Cannot reach releases.herdr.dev", result.stdout)
        self.assertFalse((local_bin / "herdr-core").exists())

    def mac_installer(self, signature, status, verify_status=0):
        home = self.bin / "home"
        (home / ".local/bin").mkdir(parents=True)
        (home / ".local/bin/herdr-core").touch()
        source = self.bin / "source/Canopy.app/Contents"
        source.mkdir(parents=True)
        plist = b"signed release plist must stay unchanged"
        (source / "Info.plist").write_bytes(plist)
        archive = self.bin / "release.tar.gz"
        with tarfile.open(archive, "w:gz") as tar:
            tar.add(source.parent, arcname="Canopy.app")
        sha = hashlib.sha256(archive.read_bytes()).hexdigest()
        sums = self.bin / "sums"
        sums.write_text(f"{sha}  canopy-macos-aarch64.tar.gz\n")
        self.stub("curl", f'''url=; out=; prev=
for a; do
  if [ "$prev" = -o ]; then out=$a; else case $a in -*) ;; *) url=$a ;; esac; fi
  prev=$a
done
case $url in */SHA256SUMS) cp "{sums}" "$out" ;; */canopy-macos-aarch64.tar.gz) cp "{archive}" "$out" ;; *) exit 97 ;; esac''')
        installed = home / "Applications/Canopy.app/Contents/Info.plist"
        installed.parent.mkdir(parents=True)
        installed.write_bytes(b"previous installed app")
        fixture = self.bin / "signature.txt"
        fixture.write_text(signature)
        log = self.bin / "codesign.log"
        self.stub("codesign", f'''echo "$*" >> "{log}"
case "$1" in
  -dvv) cat "{fixture}" >&2; exit {status} ;;
  --verify) exit {verify_status} ;;
  --force) exit 0 ;;
  *) exit 97 ;;
esac''')
        plist_log = self.bin / "plist.log"
        self.stub("PlistBuddy", f'''echo "$*" >> "{plist_log}"
printf 'dev PATH added' >> "$3"''')
        # Remap only the system tools in a scratch copy; production uses absolute paths.
        script = (ROOT / "distribution/install.sh").read_text()
        self.assertNotIn("\n    codesign ", script)
        script = script.replace("/usr/bin/codesign", str(self.bin / "codesign"))
        script = script.replace("/usr/libexec/PlistBuddy", str(self.bin / "PlistBuddy"))
        installer = self.bin / "install.sh"
        installer.write_text(script)
        env = dict(os.environ, PATH=f"{self.bin}:{os.environ['PATH']}", TEST_OS="Darwin",
                   TEST_ARCH="arm64", HOME=str(home), CANOPY_RELEASES_URL="https://releases.example")
        result = subprocess.run(["sh", str(installer)], cwd=ROOT, env=env,
                                capture_output=True, text=True, check=False)
        return result, installed.read_bytes(), log.read_text().splitlines(), plist_log.exists(), plist

    def test_mac_installer_preserves_certificate_signed_bundle(self):
        fixture = (ROOT / "scripts/fixtures/codesign/developer-id.txt").read_text()
        result, installed, calls, edited, plist = self.mac_installer(fixture, 0)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(installed, plist)
        self.assertFalse(edited)
        self.assertEqual(len(calls), 2)
        self.assertTrue(calls[0].startswith("-dvv "))
        self.assertTrue(calls[1].startswith("--verify --deep --strict "))

    def test_mac_installer_prepares_adhoc_bundle(self):
        self.check_dev_installer("adhoc", 0)

    def test_mac_installer_prepares_unsigned_bundle(self):
        self.check_dev_installer("unsigned", 1)

    def check_dev_installer(self, fixture_name, status):
        fixture = (ROOT / f"scripts/fixtures/codesign/{fixture_name}.txt").read_text()
        result, installed, calls, edited, plist = self.mac_installer(fixture, status)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(installed, plist + b"dev PATH added")
        self.assertTrue(edited)
        self.assertEqual(len(calls), 2)
        self.assertTrue(calls[1].startswith("--force --sign - "))

    def test_mac_installer_unknown_signature_leaves_installed_app_untouched(self):
        result, installed, calls, edited, _ = self.mac_installer("codesign: internal error", 1)
        self.assertEqual(result.returncode, 1)
        self.assertIn("cannot determine app signature: codesign: internal error", result.stderr)
        self.assertEqual(installed, b"previous installed app")
        self.assertFalse(edited)
        self.assertEqual(len(calls), 1)
        self.assertTrue(calls[0].startswith("-dvv "))

    def test_mac_installer_invalid_release_leaves_installed_app_untouched(self):
        fixture = (ROOT / "scripts/fixtures/codesign/developer-id.txt").read_text()
        result, installed, calls, edited, _ = self.mac_installer(fixture, 0, verify_status=1)
        self.assertEqual(result.returncode, 1)
        self.assertIn("invalid app signature", result.stderr)
        self.assertEqual(installed, b"previous installed app")
        self.assertFalse(edited)
        self.assertEqual(len(calls), 2)
        self.assertTrue(calls[1].startswith("--verify --deep --strict "))

    def test_intel_mac_package_stops_before_build(self):
        for args in ((), ("x86_64-apple-darwin",)):
            with self.subTest(args=args):
                result = self.run_script("scripts/package", "Darwin", "x86_64", *args)
                self.assertEqual(result.returncode, 1)
                self.assertIn("Intel Macs aren't supported; build from source", result.stderr)
                self.assertNotIn("unexpected build", result.stderr)

    def test_supported_installers_still_try_download(self):
        for os_name, arch in (("Darwin", "arm64"), ("Linux", "x86_64"), ("Linux", "aarch64")):
            with self.subTest(os=os_name, arch=arch):
                result = self.run_script("distribution/install.sh", os_name, arch)
                self.assertEqual(result.returncode, 1)
                self.assertIn("unexpected download", result.stderr)
                self.assertNotIn("Intel Macs", result.stderr)

    def test_packages_keep_full_notices_on_every_platform(self):
        checkout = self.bin / "checkout"
        for path in [
            "Cargo.toml", "LICENSE", "NOTICE", "THIRD-PARTY-NOTICES", "scripts/package",
            "assets/icons/canopy.svg", "assets/icons/Canopy.icns",
        ]:
            destination = checkout / path
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(ROOT / path, destination)
        sign = checkout / "scripts/mac-sign"
        sign.write_text("#!/bin/sh\nexit 0\n")
        sign.chmod(0o755)
        release = checkout / "target/release"
        release.mkdir(parents=True)
        for name in ("canopy", "canopy.exe"):
            (release / name).write_bytes(b"test binary")
        self.stub("cargo", "exit 0")
        for os_name, arch, target, prefix in (
            ("Linux", "x86_64", "linux-x86_64", "canopy"),
            ("Darwin", "arm64", "macos-aarch64", "Canopy.app/Contents/Resources"),
            ("MINGW64_NT", "x86_64", "windows-x86_64", "canopy"),
        ):
            with self.subTest(target=target):
                result = self.run_script(checkout / "scripts/package", os_name, arch)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                with tarfile.open(checkout / f"dist/canopy-{target}.tar.gz") as archive:
                    for name in ("LICENSE", "NOTICE", "THIRD-PARTY-NOTICES"):
                        self.assertEqual(
                            archive.extractfile(f"{prefix}/{name}").read(),
                            (ROOT / name).read_bytes(),
                        )
        self.stub("cargo", 'echo "unexpected build" >&2; exit 97')
        for name in ("LICENSE", "NOTICE", "THIRD-PARTY-NOTICES"):
            notice = checkout / name
            original = notice.read_bytes()
            for state in ("empty", "missing"):
                with self.subTest(notice=name, state=state):
                    if state == "empty":
                        notice.write_bytes(b"")
                    else:
                        notice.unlink()
                    result = self.run_script(checkout / "scripts/package", "Linux", "x86_64")
                    self.assertEqual(result.returncode, 1)
                    self.assertIn(f"{name} is missing or empty", result.stderr)
                    self.assertNotIn("unexpected build", result.stderr)
            notice.write_bytes(original)

    def test_supported_packages_still_try_build(self):
        for os_name, arch in (("Darwin", "arm64"), ("Linux", "x86_64"), ("Linux", "aarch64")):
            with self.subTest(os=os_name, arch=arch):
                result = self.run_script("scripts/package", os_name, arch)
                self.assertEqual(result.returncode, 97)
                self.assertIn("unexpected build", result.stderr)
                self.assertNotIn("Intel Macs", result.stderr)


if __name__ == "__main__":
    unittest.main()
