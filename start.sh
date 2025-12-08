#!/bin/bash
pkill -f wrd-server 2>/dev/null
cd "$(dirname "$0")"
./target/release/wrd-server -c config.toml
