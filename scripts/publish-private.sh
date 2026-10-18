#!/usr/bin/env bash
# Creates and publishes Leafview as a *private* GitHub repository.
set -euo pipefail
cd "$(dirname "$0")/.."
command -v gh >/dev/null || { echo 'Install gh: sudo apt install gh' >&2; exit 1; }
gh auth status >/dev/null || { echo 'Authenticate with: gh auth login' >&2; exit 1; }
repo="${1:-leafview}"
if gh repo view "$repo" >/dev/null 2>&1; then
  echo "Repository '$repo' already exists; refusing to modify it." >&2
  exit 1
fi
if [[ ! -d .git ]]; then git init -b main; fi
# Lockfile is generated locally, not guessed; build requires Cargo.
if command -v cargo >/dev/null; then
  cargo fmt --all -- --check
  cargo check --locked
  cargo test --locked
  cargo clippy --locked --all-targets -- -D warnings
  cargo build --locked --release
  python3 scripts/pty-smoke.py
else
  echo 'Cargo not available: refusing to publish uncompiled source.' >&2
  exit 1
fi
git add Cargo.toml Cargo.lock src tests scripts .github .gitignore README.md VERIFICATION.md LICENSE
git commit -m "Initial Leafview terminal PDF viewer"
gh repo create "$repo" --private --source . --remote origin --push
echo "Private repository created: $(gh repo view --json url -q .url)"
