#!/usr/bin/env bash
set -euo pipefail

REMOTE="root@unsuspicious.org"
REPO_DIR="/var/www/unsuspicious.org"
HRML_REPO="https://github.com/Unsuspicious-Industries/hrml.git"

echo "==> Updating xrml binary..."
ssh "$REMOTE" "mkdir -p ~/.cargo && echo -e '[net]\ngit-fetch-with-cli = true' > ~/.cargo/config.toml && cargo install --git $HRML_REPO xrml"

echo "==> Pulling latest usi changes..."
ssh "$REMOTE" "cd $REPO_DIR && git pull"

echo "==> Rebuilding site..."
ssh "$REMOTE" "cd $REPO_DIR && xrml build projects/usi"

echo "==> Restarting service..."
ssh "$REMOTE" "systemctl restart xrml"

echo "==> Done. Site deployed to https://unsuspicious.org"
