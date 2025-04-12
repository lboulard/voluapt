#!/usr/bin/env bash
set -e

TAG_NAME="$1"
# Determine if this is a dev build or a tagged release.
IS_DEV="$([[ "$TAG_NAME" == "dev" ]] && echo true || echo false)"

# Create working directories.
mkdir -p release-assets unpacked

echo "✔️ Copy/extract artifacts"
# Copy/extract artifacts into 'unpacked' (keep permission bits)
find artifacts -type f | while read -r artifact; do
  case "$artifact" in
  *.tar) tar xf "$artifact" -C unpacked/ ;;
  *) cp -a "$artifact" unpacked/ ;;
  esac
done

cd unpacked
ls -l

echo "✔️ Creating release archives and checksums..."
: >../release-assets/SHA256SUMS.txt

for file in *; do
  # Process only executable files or Windows executables.
  [[ -x "$file" || "$file" == *.exe ]] || continue

  echo "ℹ️ Processing $file"

  # Use the file name structure to generate a base archive name.
  # Here we assume filenames follow a pattern such as "voluapt-linux-x64" or "voluapt-win-x64.exe".
  NAME="${file%%-*}"
  PLATFORM="${file#*-}"
  PLATFORM="${PLATFORM%.exe}"

  # For non-Windows binaries, strip debug symbols.
  if [[ "$file" != *.exe ]]; then
    echo "♻️ Stripping $file"
    case "$PLATFORM" in
    *-x64*) strip "$file" || echo "⚠️ Warning: strip failed on $file" ;;
    *-aarch64*) aarch64-linux-gnu-strip "$file" || echo "⚠️ Warning: strip failed on $file" ;;
    *) echo "⚠️ Warning: strip not possible on $file for $PLATFORM" ;;
    esac
  fi

  if [[ "$IS_DEV" == true ]]; then
    ARCHIVE_BASENAME="${NAME}-dev-${PLATFORM}"
  else
    ARCHIVE_BASENAME="${NAME}-${TAG_NAME}-${PLATFORM}"
  fi

  DOCS=""
  [[ -f ../README.md ]] && DOCS="../README.md"
  [[ -f ../LICENSE.txt ]] && DOCS="$DOCS ../LICENSE.txt"

  if [[ "$file" == *.exe ]]; then
    # For Windows executables: use zip.
    ARCHIVE_NAME="${ARCHIVE_BASENAME}.zip"
    # Create the base archive with binary, its SHA file, and docs (with flattened structure).
    mv "$file" "${NAME}.exe"
    zip -j "../release-assets/${ARCHIVE_NAME}" "${NAME}.exe" $DOCS
    rm -fr "${NAME}.exe"
    (cd .. && zip -r "release-assets/${ARCHIVE_NAME}" lua)
  else
    # For non-Windows targets: use tar.gz.
    ARCHIVE_NAME="${ARCHIVE_BASENAME}.tar,gz"
    mv "$file" "$NAME"
    tar -czvf "../release-assets/${ARCHIVE_NAME}" "$NAME" $DOCS -C .. lua
    rm -fr "$NAME"
  fi

  # Generate a per-file SHA256 file and append checksum to a global SHA256SUMS.txt.
  (
    set -e
    cd ../release-assets
    sha256sum "${ARCHIVE_NAME}" | tee "${ARCHIVE_NAME}.sha256" >>SHA256SUMS.txt
  )

  echo "✅ $ARCHIVE_NAME"

done

cd ..
