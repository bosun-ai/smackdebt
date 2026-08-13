#!/usr/bin/env python3
import json
import resource
import subprocess
import sys

stderr_path, quiet, *command = sys.argv[1:]
result = subprocess.run(
    command,
    stdout=subprocess.DEVNULL if quiet == "true" else None,
    stderr=subprocess.PIPE,
    check=False,
)
resident = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
if sys.platform != "darwin":
    resident *= 1024
with open(stderr_path, "wb") as stream:
    stream.write(result.stderr)
    stream.write(
        ("smackdebt runner stats: " + json.dumps({"peak_resident_bytes": resident}, separators=(",", ":")) + "\n").encode()
    )
raise SystemExit(result.returncode)
