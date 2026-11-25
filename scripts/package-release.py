#!/usr/bin/env python3
"""生成设备发布包、对应源码与校验文件"""

import hashlib
import json
import os
import shutil
import subprocess
import tarfile
from pathlib import Path

from debian import dpkg_deb

ROOT = Path(__file__).resolve().parent.parent
DIST = ROOT / "dist"


def command(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def copy(source: Path, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)


def digest(path: Path) -> str:
    sha = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            sha.update(chunk)
    return sha.hexdigest()


def main() -> None:
    manifest = json.loads((ROOT / "app-builder.json").read_text(encoding="utf-8"))
    version = manifest["version"]
    tag = os.environ.get("RELEASE_TAG")
    if tag and tag != f"v{version}":
        raise SystemExit(f"标签 {tag} 与 app-builder.json 版本 {version} 不一致")

    target = ROOT / "crates/target/release"
    binaries = {
        "cardshot-ui": target / "cardshot-ui",
        "cardshotd": target / "cardshotd",
    }
    for name, path in binaries.items():
        if not path.is_file():
            raise SystemExit(f"缺少构建产物：{path}")

    package = DIST / f"cardshot_{version}_arm64"
    if package.exists():
        shutil.rmtree(package)
    app = package / "usr/share/APPLaunch"
    lib = package / "usr/lib/cardshot"
    unit = package / "usr/lib/cardshot"
    doc = package / "usr/share/doc/cardshot"

    copy(binaries["cardshot-ui"], app / "bin/cardshot-ui")
    copy(ROOT / "packaging/bin/run-cardshot", app / "bin/run-cardshot")
    copy(ROOT / "packaging/applications/cardshot.desktop", app / "applications/cardshot.desktop")
    for icon in (ROOT / "packaging/share/images").glob("*.png"):
        copy(icon, app / "share/images" / icon.name)

    copy(binaries["cardshotd"], lib / "cardshotd")
    shutil.copytree(ROOT / "service/web", lib / "web")
    unit.mkdir(parents=True, exist_ok=True)
    (unit / "cardshot.service").write_text(
        "\n".join([
            "[Unit]",
            "Description=CardShot screenshot and recording service",
            "After=default.target",
            "StartLimitIntervalSec=0",
            "",
            "[Service]",
            "Type=simple",
            "WorkingDirectory=/usr/lib/cardshot",
            "ExecStart=/usr/lib/cardshot/cardshotd --config %h/.config/cardshot/config.json",
            "Restart=always",
            "RestartSec=2",
            "",
            "[Install]",
            "WantedBy=default.target",
            "",
        ]),
        encoding="utf-8",
    )
    (unit / "cardshot.service").chmod(0o644)
    copy(ROOT / "README.md", doc / "README.md")
    copy(ROOT / "docs/feasibility.md", doc / "feasibility.md")

    control = package / "DEBIAN/control"
    control.parent.mkdir(parents=True, exist_ok=True)
    control.write_text(
        "\n".join([
            f"Package: {manifest['package_name']}",
            f"Version: {version}",
            "Architecture: arm64",
            "Maintainer: journey-ad <journey-ad@users.noreply.github.com>",
            "Section: utils",
            "Priority: optional",
            f"Description: {manifest['store']['summary']}",
        ]) + "\n",
        encoding="utf-8",
    )
    postinst = package / "DEBIAN/postinst"
    postinst.write_text(
        "#!/bin/sh\n"
        "set -e\n"
        "UNIT=/usr/lib/cardshot/cardshot.service\n"
        "for entry in $(getent passwd | awk -F: '$3 >= 1000 && $3 < 60000 && $6 ~ /^\\/home\\// { print $1 \":\" $6 }'); do\n"
        "  user=${entry%%:*}\n"
        "  home=${entry#*:}\n"
        "  dir=\"$home/.config/systemd/user\"\n"
        "  mkdir -p \"$dir/default.target.wants\"\n"
        "  ln -sf \"$UNIT\" \"$dir/cardshot.service\"\n"
        "  ln -sf \"$UNIT\" \"$dir/default.target.wants/cardshot.service\"\n"
        "  uid=$(id -u \"$user\")\n"
        "  chown -R \"$user\" \"$home/.config/systemd/user\" 2>/dev/null || true\n"
        "  runuser -u \"$user\" -- env XDG_RUNTIME_DIR=\"/run/user/$uid\" systemctl --user daemon-reload >/dev/null 2>&1 || true\n"
        "  runuser -u \"$user\" -- env XDG_RUNTIME_DIR=\"/run/user/$uid\" systemctl --user start cardshot.service >/dev/null 2>&1 || true\n"
        "done\n"
        "exit 0\n"
    )
    postinst.chmod(0o755)
    dpkg_deb("--build", "--root-owner-group", package, DIST / f"cardshot_{version}_arm64.deb")

    source = DIST / f"cardshot-{version}-source"
    if source.exists():
        shutil.rmtree(source)
    source.mkdir(parents=True)
    archive = DIST / f"cardshot-{version}-source.tar.gz"
    with tarfile.open(archive, "w:gz") as bundle:
        bundle.add(ROOT / "crates", arcname=f"cardshot-{version}-source/crates",
                   filter=lambda info: None if "/target" in info.name else info)
        for name in ("app-builder.json", "README.md", "packaging", "service", "scripts", "docs"):
            bundle.add(ROOT / name, arcname=f"cardshot-{version}-source/{name}")
    shutil.rmtree(source)

    deb = DIST / f"cardshot_{version}_arm64.deb"
    lines = []
    for path in sorted([deb, archive]):
        lines.append(f"{digest(path)}  {path.name}")
    (DIST / "SHA256SUMS").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"生成 {deb.name} 与 {archive.name}")


if __name__ == "__main__":
    main()
