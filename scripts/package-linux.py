#!/usr/bin/env python3
"""Build Ubuntu 26.04 amd64 deb and a relocatable type-2 AppImage, without root."""
import argparse
import contextlib
import hashlib
import io
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tempfile
import tomllib
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
TARGET = ROOT / "target/linux-packaging"
DIST = ROOT / "dist"
REQUIRED_HOST_COMMANDS = json.loads((ROOT / "packaging/linux/tools.json").read_text())["required_host_commands"]
# Host ABI, graphics vendors, fonts and desktop services remain system-owned.
# libmpv, FFmpeg, libplacebo, sound clients and their non-driver dependencies bundle.
HOST_LIBS = re.compile(r"^(ld-linux.*|lib(c|m|pthread|dl|rt|resolv)\.so\..*|lib(GL|GLX|GLdispatch|EGL|OpenGL|GLESv[12])\.so\..*|lib(nvidia|cuda|nvcuvid).*|libvulkan\.so\..*|libdrm[^/]*\.so\..*|libgbm\.so\..*)$")

def run(args, **kwargs):
    return subprocess.run([str(a) for a in args], check=True, **kwargs)

def output(args):
    return subprocess.check_output([str(a) for a in args], text=True).strip()

def git_command(*args):
    # CI containers can run as a different owner from the mounted checkout.
    # Trust only this script's repository for this command, without modifying
    # global configuration or trusting every repository via safe.directory=*.
    return ["git", "-c", f"safe.directory={ROOT}", "-C", str(ROOT), *args]

def check_host_tools():
    tools = {name: shutil.which(name) for name in REQUIRED_HOST_COMMANDS}
    missing = [name for name, path in tools.items() if path is None]
    if missing:
        packages = sorted({REQUIRED_HOST_COMMANDS[name] for name in missing if REQUIRED_HOST_COMMANDS[name]})
        message = "缺少打包工具：" + ", ".join(missing)
        if packages:
            message += "\n所需 apt 包：" + " ".join(packages)
        if any(REQUIRED_HOST_COMMANDS[name] is None for name in missing):
            message += "\n请安装 Rust 工具链并将 cargo / rustc 加入 PATH。"
        raise RuntimeError(message)
    return tools

def digest(path):
    with Path(path).open("rb") as file:
        return hashlib.file_digest(file, "sha256").hexdigest()

def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")

def system_mpv():
    for path in [Path("/usr/lib/x86_64-linux-gnu/libmpv.so.2"), Path("/lib/x86_64-linux-gnu/libmpv.so.2")]:
        if path.is_file():
            return path.resolve()
    raise RuntimeError("libmpv2 未安装；先运行 scripts/check-linux-deps.py")

def common(root, binary):
    (root / "usr/bin").mkdir(parents=True)
    shutil.copy2(binary, root / "usr/bin/yyplayer")
    desktop = root / "usr/share/applications/yyplayer.desktop"
    desktop.parent.mkdir(parents=True)
    shutil.copy2(ROOT / "packaging/linux/yyplayer.desktop", desktop)
    run(["desktop-file-validate", desktop])
    for size in [16, 32, 48, 64, 128, 256, 512]:
        dest = root / f"usr/share/icons/hicolor/{size}x{size}/apps/yyplayer.png"
        dest.parent.mkdir(parents=True)
        shutil.copy2(ROOT / f"assets/icons/yyplayer/liquid-orbit-disc/yyplayer-{size}.png", dest)
    docs = root / "usr/share/doc/yyplayer"
    docs.mkdir(parents=True)
    for name in ["LICENSE", "README.md", "THIRD_PARTY_NOTICES.md"]:
        shutil.copy2(ROOT / name, docs / name)
    shutil.copy2(ROOT / "LICENSE", docs / "copyright")
    shutil.copytree("/usr/share/common-licenses", root / "usr/share/common-licenses", symlinks=False)
    shutil.copy2(ROOT / "docs/LINUX.md", docs / "LINUX.md")
    shutil.copy2(ROOT / "docs/ONLINE_API.md", docs / "ONLINE_API.md")
    cloud_api = root / "usr/share/yyplayer/cloud_api"
    cloud_api.mkdir(parents=True, exist_ok=True)
    for name in ("README.md", "example-gd.json"):
        shutil.copy2(ROOT / "config/cloud_api" / name, cloud_api / name)
    return docs

def dependencies(binary):
    text = output(["ldd", binary])
    if "not found" in text:
        raise RuntimeError(f"ELF dependency missing for {binary}:\n{text}")
    paths = []
    for line in text.splitlines():
        match = re.search(r"=>\s+(/\S+)\s+\(", line) or re.search(r"^\s*(/\S+)\s+\(", line)
        if match:
            paths.append(Path(match[1]))
    return paths

def owner(path):
    candidates = [path, path.resolve()]
    for candidate in candidates:
        result = subprocess.run(["dpkg-query", "-S", str(candidate)], capture_output=True, text=True)
        if result.returncode == 0:
            return result.stdout.split(": ", 1)[0].splitlines()[0]
    raise RuntimeError(f"Cannot establish distribution source/license for {path}")

def bundle(appdir, binary, mpv):
    libdir = appdir / "usr/lib/yyplayer"
    libdir.mkdir(parents=True)
    runtime = appdir / "usr/bin/runtime"
    runtime.mkdir()
    shutil.copy2(mpv, runtime / "libmpv.so.2")
    queue = [binary, mpv]
    plugin_paths = {}
    for dirname in ["spa-0.2", "pipewire-0.3"]:
        directory = Path("/usr/lib/x86_64-linux-gnu") / dirname
        if directory.is_dir():
            for plugin in directory.rglob("*.so"):
                plugin_paths[plugin.resolve()] = libdir / dirname / plugin.relative_to(directory)
                queue.append(plugin)
    # These are dlopened, not represented by the application's DT_NEEDED table.
    queue += [Path("/usr/lib/x86_64-linux-gnu") / name for name in ["libfontconfig.so.1", "libxkbcommon.so.0", "libxkbcommon-x11.so.0", "libX11.so.6", "libX11-xcb.so.1", "libXcursor.so.1", "libXi.so.6", "libXrandr.so.2", "libwayland-client.so.0", "libwayland-cursor.so.0", "libwayland-egl.so.1"]]
    visited, packages, entries = set(), set(), []
    while queue:
        source = queue.pop()
        if not source.is_file():
            raise RuntimeError(f"Runtime library missing: {source}")
        real = source.resolve()
        if real in visited:
            continue
        visited.add(real)
        if HOST_LIBS.fullmatch(source.name):
            entries.append({"library": source.name, "policy": "host graphics/ABI"})
            continue
        if real != binary.resolve():
            packages.add(owner(source))
        for dep in dependencies(real):
            queue.append(dep)
        if real in [binary.resolve(), mpv.resolve()]:
            continue
        soname = output(["patchelf", "--print-soname", real]) or source.name
        dest = plugin_paths.get(real, libdir / soname)
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(real, dest)
        # All bundled ELF entries resolve peers within the relocatable AppDir.
        run(["patchelf", "--set-rpath", "$ORIGIN" + ":$ORIGIN/" + os.path.relpath(libdir, dest.parent), dest])
        entries.append({"library": soname, "original_sha256": digest(real), "bundled_sha256": digest(dest), "package": owner(source)})
    run(["patchelf", "--set-rpath", "$ORIGIN/../lib/yyplayer", appdir / "usr/bin/yyplayer"])
    run(["patchelf", "--set-rpath", "$ORIGIN/../../lib/yyplayer", runtime / "libmpv.so.2"])
    version = output(["dpkg-query", "-W", "-f=${Version}", "libmpv2"])
    write_json(runtime / "manifest.json", {"platform": "linux-x86_64", "library": "libmpv.so.2", "client_api_major": 2, "package": "libmpv2", "version": version, "library_sha256": digest(runtime / "libmpv.so.2")})
    records = []
    for package in sorted(packages):
        fields = output(["dpkg-query", "-W", "-f=${binary:Package}\t${Version}\t${source:Package}\t${source:Version}", package]).split("\t")
        name, version, src, srcversion = fields
        source_url = "https://launchpad.net/ubuntu/+source/" + urllib.parse.quote(src, safe="") + "/" + urllib.parse.quote(srcversion, safe="")
        record = {"package": name, "version": version, "source_package": src, "source_version": srcversion, "corresponding_source": source_url}
        records.append(record)
        copyright = Path("/usr/share/doc") / name.split(":")[0] / "copyright"
        if not copyright.is_file():
            raise RuntimeError(f"Missing copyright for {name}")
        dest = appdir / "usr/share/doc/yyplayer/runtime-notices" / name.replace(":", "_")
        dest.mkdir(parents=True)
        shutil.copy2(copyright, dest / "copyright")
    # PipeWire may load its SPA audio plugins from an installation-specific path.
    # Bundle the matching plugin tree and point only this AppRun to it.
    configuration = Path("/usr/share/pipewire")
    if configuration.is_dir():
        shutil.copytree(configuration, appdir / "usr/share/pipewire", symlinks=False)
    write_json(appdir / "usr/share/doc/yyplayer/runtime-manifest.json", {"elf": entries, "source_packages": records})

def source_kit(binary, offline):
    # Includes the actual working tree, not merely HEAD. Git, media, cache and
    # user configuration are excluded. Locked Rust dependency source archives
    # are provided beside the artifacts in the same source kit.
    kit = DIST / "YYPlayer-linux-source.tar.gz"
    names = subprocess.check_output(git_command("ls-files", "--cached", "--others", "--exclude-standard", "-z"))
    tracked = [os.fsdecode(name) for name in names.split(b"\0") if name]
    # Cargo resolves conditional manifests for other platforms even when the
    # eventual build target is Linux. Include the entire locked graph so a
    # completely empty Cargo home can resolve it without an index or network.
    metadata = json.loads(output(["cargo", "metadata", "--locked", "--offline", "--format-version", "1"]))
    registry = []
    for package in metadata["packages"]:
        if package["source"] and package["source"].startswith("registry+"):
            registry.append(package)
    rust_sources = []
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    with tarfile.open(kit, "w:gz") as tar:
        for name in sorted(set(tracked)):
            path = ROOT / name
            if path.is_file() and not name.startswith(("target/", "dist/", ".git/")):
                tar.add(path, arcname="yyplayer/" + name, recursive=False)
        for package in registry:
            name = f"{package['name']}-{package['version']}.crate"
            matches = list((cargo_home / "registry/cache").glob("*/" + name))
            if not matches:
                raise RuntimeError(f"Locked Rust source archive missing: {name}")
            tar.add(matches[0], arcname="rust-crates/" + name)
            rust_sources.append({"name": package["name"], "version": package["version"], "license": package["license"], "sha256": digest(matches[0]), "archive": "rust-crates/" + name})
        data = json.dumps(rust_sources, indent=2).encode()
        entry = tarfile.TarInfo("rust-crates/manifest.json")
        entry.size = len(data)
        tar.addfile(entry, io.BytesIO(data))
        runtime_source = TARGET / "tools/type2-runtime-source.tar.gz"
        spec = json.loads((ROOT / "packaging/linux/tools.json").read_text())["appimagetool"]["runtime_source_archive"]
        if not runtime_source.exists():
            if offline:
                raise RuntimeError("Fixed AppImage runtime source archive not cached")
            with urllib.request.urlopen(spec["url"], timeout=60) as response, runtime_source.open("wb") as dest:
                shutil.copyfileobj(response, dest)
        if digest(runtime_source) != spec["sha256"]:
            raise RuntimeError("AppImage runtime source archive digest differs")
        tar.add(runtime_source, arcname="appimage-runtime/type2-runtime-source.tar.gz")
    write_json(DIST / "YYPlayer-linux-rust-sources.json", rust_sources)
    return kit

def rust_notices(docs):
    metadata = json.loads(output(["cargo", "metadata", "--locked", "--offline", "--format-version", "1"]))
    records = []
    for package in metadata["packages"]:
        if not package["source"]:
            continue
        source = Path(package["manifest_path"]).parent
        copied = []
        for path in source.rglob("*"):
            if path.is_file() and (path.name.lower().startswith(("license", "copyright", "copying")) or path.name.lower() in ["notice", "notice.txt", "notice.md"]):
                relative = path.relative_to(source)
                dest = docs / "rust-notices" / (package["name"] + "-" + package["version"]) / relative
                dest.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(path, dest)
                copied.append(str(relative))
        records.append({"crate": package["name"], "version": package["version"], "license": package["license"], "notice_files": copied})
    write_json(docs / "rust-notices/manifest.json", records)

@contextlib.contextmanager
def appimage_tool(offline):
    info = json.loads((ROOT / "packaging/linux/tools.json").read_text())["appimagetool"]
    tool = TARGET / "tools/appimagetool.AppImage"
    tool.parent.mkdir(parents=True, exist_ok=True)
    if not tool.is_file():
        if offline:
            raise RuntimeError(f"Download verified appimagetool first: {info['url']} -> {tool}")
        with urllib.request.urlopen(info["url"], timeout=60) as response, tool.open("wb") as dest:
            shutil.copyfileobj(response, dest)
    if digest(tool) != info["sha256"]:
        raise RuntimeError("appimagetool SHA-256 mismatch")
    tool.chmod(0o755)
    # Always execute files freshly extracted from the verified archive, not a
    # potentially stale or modified unpacked cache.
    with tempfile.TemporaryDirectory(prefix="verified-tool-", dir=tool.parent) as directory:
        directory = Path(directory)
        run([tool, "--appimage-extract"], cwd=directory, stdout=subprocess.DEVNULL)
        offset = int(output([tool, "--appimage-offset"]))
        runtime = directory / "type2-runtime"
        with tool.open("rb") as src, runtime.open("wb") as dest:
            dest.write(src.read(offset))
        yield directory / "squashfs-root/AppRun", runtime

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/yyplayer")
    parser.add_argument("--offline", action="store_true", help="No Cargo or tool downloads; requires complete caches")
    parser.add_argument("--skip-build", action="store_true")
    parser.add_argument("--check-tools", action="store_true", help="Only check required host commands; no build, download or packaging")
    args = parser.parse_args()
    check_host_tools()
    if args.check_tools:
        print("deb / AppImage 宿主打包工具检查通过。")
        return
    if output(["dpkg", "--print-architecture"]) != "amd64":
        raise RuntimeError("本轮包仅资格验证 Ubuntu amd64")
    # Check repository access before the potentially expensive Release build.
    revision = output(git_command("rev-parse", "--short=12", "HEAD"))
    # Build only fetches host dependencies, but notices / corresponding source
    # require the complete locked graph, including other platforms. Do not use
    # --target here. In offline mode this verifies the cache before packaging.
    run(["cargo", "fetch", "--locked"] + (["--offline"] if args.offline else []), cwd=ROOT)
    if not args.skip_build:
        run(["cargo", "build", "--locked", "--release", "-p", "yyplayer-app", "--bin", "yyplayer"] + (["--offline"] if args.offline else []), cwd=ROOT)
    binary = args.binary.resolve()
    if not binary.is_file():
        raise RuntimeError(f"Release executable missing: {binary}")
    version = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    package_version = version + "+linux." + revision
    DIST.mkdir(exist_ok=True)
    TARGET.mkdir(parents=True, exist_ok=True)
    mpv = system_mpv()
    # Each staging directory is uniquely created within target; never deletes an
    # arbitrary path or an installed/user configuration directory.
    with tempfile.TemporaryDirectory(prefix="package-", dir=TARGET) as stage:
        stage = Path(stage)
        qualified_binary = stage / "qualified-release"
        shutil.copy2(binary, qualified_binary)
        binary = qualified_binary
        deb = stage / "deb"
        docs = common(deb, binary)
        rust_notices(docs)
        info = {"application": "YYPlayer", "version": package_version, "base_commit": revision, "dirty": bool(output(git_command("status", "--porcelain"))), "target": "x86_64-unknown-linux-gnu", "host": Path("/etc/os-release").read_text(), "rustc": output(["rustc", "--version"]), "binary_sha256": digest(binary), "libmpv_package": output(["dpkg-query", "-W", "-f=${Version}", "libmpv2"]), "libmpv_original_sha256": digest(mpv)}
        write_json(docs / "BUILD-INFO.json", info)
        (deb / "DEBIAN").mkdir()
        libc_version = output(["dpkg-query", "-W", "-f=${Version}", "libc6"]).split("-")[0]
        depends = f"libc6 (>= {libc_version}), libgcc-s1, libmpv2 (>= 0.41.0), libfontconfig1, libxkbcommon0, libegl1, libgl1, libwayland-client0, libwayland-cursor0, libwayland-egl1, libx11-6, libx11-xcb1, libxcursor1, libxi6, libxrandr2, libxkbcommon-x11-0"
        installed_size = sum(p.stat().st_size for p in deb.rglob("*") if p.is_file()) // 1024
        (deb / "DEBIAN/control").write_text(f"Package: yyplayer\nVersion: {package_version}\nSection: video\nPriority: optional\nArchitecture: amd64\nMaintainer: YYPlayer contributors <yyplayer@users.noreply.github.com>\nInstalled-Size: {installed_size}\nDepends: {depends}\nRecommends: xdg-desktop-portal, xdg-desktop-portal-gnome | xdg-desktop-portal-gtk, fonts-noto-cjk\nHomepage: https://github.com/YYDanielJr/yyplayer\nDescription: Local audio and video player built with Slint and libmpv\n Music and video libraries, lyrics, EQ, native desktop dialogs and GPU video.\n Ubuntu 26.04 amd64 development preview; device qualification is documented.\n")
        deb_output = DIST / f"yyplayer_{package_version}_amd64.deb"
        run(["dpkg-deb", "--root-owner-group", "--build", deb, deb_output])
        appdir = stage / "YYPlayer.AppDir"
        docs = common(appdir, binary)
        shutil.copytree(deb / "usr/share/doc/yyplayer/rust-notices", docs / "rust-notices")
        shutil.copytree(ROOT / "packaging/linux/runtime-notices", docs / "appimage-runtime-notices")
        write_json(docs / "BUILD-INFO.json", info)
        shutil.copy2(ROOT / "packaging/linux/AppRun", appdir / "AppRun")
        (appdir / "AppRun").chmod(0o755)
        (appdir / "yyplayer.desktop").symlink_to("usr/share/applications/yyplayer.desktop")
        (appdir / "yyplayer.png").symlink_to("usr/share/icons/hicolor/256x256/apps/yyplayer.png")
        (appdir / ".DirIcon").symlink_to("yyplayer.png")
        bundle(appdir, binary, mpv)
        app_info = {**info, "bundled_binary_sha256": digest(appdir / "usr/bin/yyplayer"), "appimage_tool": json.loads((ROOT / "packaging/linux/tools.json").read_text())["appimagetool"]}
        write_json(docs / "BUILD-INFO.json", app_info)
        appimage = DIST / f"YYPlayer-{package_version}-x86_64.AppImage"
        with appimage_tool(args.offline) as (tool, runtime):
            run([tool, "--runtime-file", runtime, "--no-appstream", "--mksquashfs-opt", "-processors", "--mksquashfs-opt", "2", appdir, appimage], env={**os.environ, "ARCH": "x86_64"})
        source = source_kit(binary, args.offline)
        info["source_archive"] = source.name
        info["source_sha256"] = digest(source)
        write_json(DIST / "YYPlayer-linux-BUILD-INFO.json", info)
        shutil.copy2(docs / "runtime-manifest.json", DIST / "YYPlayer-linux-runtime-manifest.json")
        artifacts = [deb_output, appimage, source, DIST / "YYPlayer-linux-rust-sources.json", DIST / "YYPlayer-linux-runtime-manifest.json", DIST / "YYPlayer-linux-BUILD-INFO.json"]
        (DIST / "SHA256SUMS-linux").write_text("".join(f"{digest(path)}  {path.name}\n" for path in artifacts))
        for artifact in artifacts[:3]:
            print(f"Created {artifact} ({artifact.stat().st_size / 1024**2:.1f} MiB)")

if __name__ == "__main__":
    main()
