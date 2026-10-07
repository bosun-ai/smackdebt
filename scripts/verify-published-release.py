"""Verify that the public release can install its advertised binary version."""

import json
import os
from pathlib import Path
import subprocess
import tempfile


def run(*arguments, **kwargs):
    return subprocess.check_output(arguments, text=True, **kwargs).strip()


def verify(tag, repository, directory):
    release = json.loads(run("gh", "release", "view", tag, "--repo", repository,
                             "--json", "tagName,isDraft,assets"))
    assert release["tagName"] == tag and not release["isDraft"], "release is not public"
    names = {asset["name"] for asset in release["assets"]}
    expected = {"install.sh", "smackdebt-installer.sh", "smackdebt.rb", "dist-manifest.json"}
    for target in ("x86_64-unknown-linux-gnu", "x86_64-apple-darwin", "aarch64-apple-darwin"):
        expected.update({f"smackdebt-{target}.tar.xz", f"smackdebt-{target}.tar.xz.sha256"})
    assert expected <= names, f"missing release assets: {expected - names}"
    version = tag.removeprefix("v")
    home = directory / "user"
    home.mkdir()
    environment = dict(os.environ, HOME=str(home), CARGO_HOME=str(home / ".cargo"),
                       XDG_CONFIG_HOME=str(home / ".config"),
                       SMACKDEBT_INSTALL_DIR=str(home / ".cargo"), INSTALLER_NO_MODIFY_PATH="1")
    installer = directory / "install.sh"
    run("curl", "--proto", "=https", "--tlsv1.2", "-fsSL",
        f"https://github.com/{repository}/releases/download/{tag}/install.sh", "-o", str(installer))
    run("sh", str(installer), env=environment)
    assert run(str(home / ".cargo/bin/smackdebt"), "--version") == f"smackdebt {version}"
    assert not (home / ".agents").exists() and not (home / ".claude").exists()
    destination = directory / "binstall"
    run("cargo-binstall", "smackdebt", "--version", f"={version}", "--no-confirm",
        "--disable-strategies", "quick-install,compile", "--install-path", str(destination))
    assert run(str(destination / "smackdebt"), "--version") == f"smackdebt {version}"
    run(str(destination / "smackdebt"), "init", "--all", "--dry-run", env=environment)


if __name__ == "__main__":
    with tempfile.TemporaryDirectory(prefix="smackdebt-published-") as temporary:
        verify(os.environ["RELEASE_TAG"], os.environ["RELEASE_REPOSITORY"], Path(temporary))
    print("Published release: downloads and cargo-binstall passed")
