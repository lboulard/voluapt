#!/usr/bin/awk -f

# Usage: ./extract_changelog.awk TAG

BEGIN {
    if (ARGC < 2) {
        print "Usage: awk -f extract_changelog.awk TAG" > "/dev/stderr"
        exit 1
    }

    Tag = ARGV[1]
    ARGC = 1  # only process the hardcoded file

    File = "CHANGELOG.md"

    if ((getline dummy < File) < 0) {
        print "No ChangeLog found" > "/dev/stdout"
        exit 0
    }

    if (Tag == "dev") {
        Marker = "\\[Unreleased\\]$"
    } else {
        Marker = Tag " "
    }

    StartPattern = "^## " Marker
    EndPattern = "^## "

    InSection = 0

    # make sure awk processes this file
    ARGV[ARGC++] = File
}

{
    if ($0 ~ StartPattern) {
        InSection = 1
        next
    }

    if (InSection && $0 ~ EndPattern) {
        InSection = 0
        exit
    }

    if (InSection) {
        print
    }
}
