#!/usr/bin/env bash
set -e

TAG_NAME="$1"
# Determine if this is a dev build or a tagged release.
IS_DEV="$([[ "$TAG_NAME" == "dev" ]] && echo true || echo false)"

# Create working directories.
mkdir -p release-assets unpacked

# Copy built artifacts into 'unpacked'
find artifacts -type f -exec cp {} unpacked/ \;

echo "Generating SHA256 checksums and processing binaries..."
cd unpacked
: >../release-assets/SHA256SUMS.txt

for file in *; do
  # Process only executable files or Windows executables.
  [[ -x "$file" || "$file" == *.exe ]] || continue

  # Use the file name structure to generate a base archive name.
  # Here we assume filenames follow a pattern such as "voluapt-linux-x64" or "voluapt-win-x64.exe".
  NAME="${file%%-*}"
  PLATFORM="${file#${NAME}-}"

  # For non-Windows binaries, strip debug symbols.
  if [[ "$file" != *.exe ]]; then
    echo "Stripping $file"
    strip "$file" || echo "Warning: strip failed on $file"
  fi

  # Generate a per-file SHA256 file and append checksum to a global SHA256SUMS.txt.
  sha256sum "$file" | tee "$file.sha256" >>../release-assets/SHA256SUMS.txt

  if [[ "$IS_DEV" == true ]]; then
    ARCHIVE_BASENAME="${NAME}-dev-${PLATFORM}"
  else
    ARCHIVE_BASENAME="${NAME}-${TAG_NAME}-${PLATFORM}"
  fi

  DOCS=""
  [[ -f ../README.md ]] && DOCS="../README.md"
  [[ -f ../LICENSE.txt ]] && DOCS="$DOCS ../LICENSE.txt"

  (
    set -eux

    if [[ "$file" == *.exe ]]; then
      # For Windows executables: use zip.
      ARCHIVE_FILE="../release-assets/${ARCHIVE_BASENAME}.zip"
      # Create the base archive with binary, its SHA file, and docs (with flattened structure).
      zip -j "$ARCHIVE_FILE" "$file" "$file.sha256" $DOCS
      # If lua folder exists, add it preserving its structure.
      (cd .. && zip -r "release-assets/${ARCHIVE_BASENAME}.zip" lua)
    else
      # For non-Windows targets: use tar.gz.
      ARCHIVE_FILE="../release-assets/${ARCHIVE_BASENAME}.tar.gz"
      tar -czvf "$ARCHIVE_FILE" "$file" "$file.sha256" $DOCS -C .. lua
    fi
  )

done

cd ..
