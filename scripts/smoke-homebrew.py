"""Exercise the generated formula's installation using a finished local archive."""

import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
RUBY = '''
require "formula"
load ARGV[0]
formula = Smackdebt.new("smackdebt", Pathname.new(ARGV[0]), :stable)
destination = Pathname.new(ARGV[2])
formula.define_singleton_method(:prefix) { |*| destination }
Dir.chdir(ARGV[1]) { formula.install }
'''


def check(assets, target):
    with tempfile.TemporaryDirectory(prefix="smackdebt-homebrew-") as temporary:
        directory = Path(temporary)
        with tarfile.open(assets / f"smackdebt-{target}.tar.xz") as archive:
            archive.extractall(directory / "source", filter="data")
        source = directory / "source" / f"smackdebt-{target}"
        destination = directory / "installed"
        subprocess.run(["brew", "ruby", "-e", RUBY, str(assets / "smackdebt.rb"),
                        str(source), str(destination)], check=True,
                       env=dict(os.environ, HOMEBREW_NO_AUTO_UPDATE="1"))
        version = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]["version"]
        actual = subprocess.check_output([destination / "bin/smackdebt", "--version"], text=True)
        assert actual.strip() == f"smackdebt {version}", actual
    print("Homebrew: generated formula installed the finished archive")


if __name__ == "__main__":
    check(Path(sys.argv[1]).resolve(), sys.argv[2])
