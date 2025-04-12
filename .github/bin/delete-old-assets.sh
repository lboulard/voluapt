#!/usr/bin/env bash

set -e

RELEASE_TAG="$1"
GH_TOKEN="$2"

if [[ "$RELEASE_TAG" != "dev" ]]; then
  echo "Skipping deletion for tag: $RELEASE_TAG"
  exit 0
fi

echo "Cleaning up assets in 'dev' release..."

# first run will fail as dev release does not exist
ASSETS=$(gh release view dev --json assets -q '.assets[].id' || true)
for ID in $ASSETS; do
  echo "Deleting asset ID $ID"
  gh api -X DELETE /repos/"${GITHUB_REPOSITORY}"/releases/assets/$ID -H "Authorization: token $GH_TOKEN"
done
