#!/bin/sh
set -eu
exec target/release/smackdebt diff HEAD "$1" --json
