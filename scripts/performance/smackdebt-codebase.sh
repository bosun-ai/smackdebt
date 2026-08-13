#!/bin/sh
set -eu
exec target/release/smackdebt --json "$1"
