#!/bin/sh
set -eu
exec target/release/smackdebt --json --history 36500d "$1"
