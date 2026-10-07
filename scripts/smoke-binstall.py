"""Install a finished archive using the shipped binstall metadata and local HTTP."""

from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import ssl
from pathlib import Path
import subprocess
import sys
import tempfile
from threading import Thread
import tomllib

ROOT = Path(__file__).resolve().parents[1]


class ArchiveHandler(SimpleHTTPRequestHandler):
    def do_GET(self):
        try:
            super().do_GET()
        except (BrokenPipeError, ConnectionResetError):
            # Binstall probes an archive before downloading it fully.
            pass


def serve_https(server, directory):
    certificate, key = directory / "certificate.pem", directory / "key.pem"
    config = directory / "tls.cnf"
    config.write_text("[req]\nprompt = no\ndistinguished_name = dn\nx509_extensions = ext\n"
                      "[dn]\nCN = localhost\n[ext]\nsubjectAltName = IP:127.0.0.1\n"
                      "basicConstraints = critical,CA:FALSE\n")
    subprocess.run(["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "1",
                    "-config", str(config), "-keyout", str(key), "-out", str(certificate)],
                   check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain(certificate, key)
    server.socket = context.wrap_socket(server.socket, server_side=True)
    return certificate


def check(assets, target):
    workspace = tomllib.loads((ROOT / "Cargo.toml").read_text())
    package = tomllib.loads((ROOT / "crates/cli/Cargo.toml").read_text())["package"]
    version = workspace["workspace"]["package"]["version"]
    metadata = package["metadata"]["binstall"]
    handler = partial(ArchiveHandler, directory=str(assets))
    server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
    try:
        with tempfile.TemporaryDirectory(prefix="smackdebt-binstall-") as temporary:
            directory = Path(temporary)
            certificate = serve_https(server, directory)
            thread = Thread(target=server.serve_forever, daemon=True)
            thread.start()
            destination = directory / "bin"
            # Resolve the workspace inheritance without depending on unpublished crates.
            manifest = directory / "Cargo.toml"
            manifest.write_text(f'''[package]
name = "smackdebt"
version = "{version}"
repository = "https://github.com/bosun-ai/smackdebt"
[[bin]]
name = "smackdebt"
path = "main.rs"
[package.metadata.binstall]
pkg-fmt = "{metadata['pkg-fmt']}"
bin-dir = "{metadata['bin-dir']}"
''')
            (directory / "main.rs").write_text("fn main() {}")
            name = metadata["pkg-url"].rsplit("/", 1)[1]
            url = f"https://127.0.0.1:{server.server_port}/{name}"
            subprocess.run(["cargo-binstall", "smackdebt", "--manifest-path", str(manifest),
                            "--pkg-url", url, "--targets", target, "--no-confirm",
                            "--root-certificates", str(certificate),
                            "--disable-strategies", "quick-install,compile", "--no-discover-github-token",
                            "--install-path", str(destination)], check=True)
            output = subprocess.check_output([destination / "smackdebt", "--version"], text=True)
            assert output.strip() == f"smackdebt {version}", output
    finally:
        if "thread" in locals():
            server.shutdown()
            thread.join()
        server.server_close()
    print("cargo-binstall: finished archive installed successfully")


if __name__ == "__main__":
    check(Path(sys.argv[1]).resolve(), sys.argv[2])
