#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT_DIR/nebula-aura-1.0.0-source.zip"
cd "$ROOT_DIR"
rm -f "$OUT"
zip -r "$OUT" . -x '.git/*' 'target/*'
echo "Created $OUT"
