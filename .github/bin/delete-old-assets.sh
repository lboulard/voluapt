#!/usr/bin/env bash

set -e

RELEASE_TAG="$1"
GH_TOKEN="$2"

if [[ "$RELEASE_TAG" != "dev" ]]; then
  echo "⚠️ Skipping deletion for tag: $RELEASE_TAG"
  exit 0
fi

echo "ℹ️ Cleaning up assets in 'dev' release..."

# first run will fail as dev release does not exist
ASSETS=$(gh release view dev --json assets -q '.assets[].apiUrl' || true)
for URL in $ASSETS; do
  echo "♻️ Deleting asset $URL"
  gh api -X DELETE -H "Authorization: token $GH_TOKEN" "$URL"
done
