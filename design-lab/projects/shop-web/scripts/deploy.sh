#!/bin/bash
set -euo pipefail
npm run build
rsync -az --delete dist/ "$DEPLOY_HOST:/srv/shop/"
