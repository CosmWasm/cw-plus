#!/bin/bash
set -o errexit -o pipefail

ORIGINAL_OPTS=$*
OPTS=$(getopt -l "help,since-tag:,upcoming-tag:,full,token:" -o "hu:ft" -- "$@") || exit 1

function print_usage() {
    echo -e "Usage: $0 [-h|--help] [-f|--full] [--since-tag <tag>] [-u|--upcoming-tag] <tag> [-t|--token <token>]
-h, --help               Display help
-f, --full               Process changes since the beginning (by default: since latest git version tag)
--since-tag <tag>        Process changes since git version tag <tag> (by default: since latest git version tag)
-u, --upcoming-tag <tag> Add a <tag> title in CHANGELOG for the new changes
--token <token>          Pass changelog github token <token>"
}

function remove_opt() {
    ORIGINAL_OPTS=$(echo "$ORIGINAL_OPTS" | sed "s/\\B$1\\b//")
}

eval set -- "$OPTS"
while true
do
case $1 in
  -h|--help)
    print_usage
    exit 0
    ;;
  --since-tag)
    shift
    TAG="$1"
    ;;
  -f|--full)
    TAG="<FULL>"
    remove_opt $1
    ;;
  -u|--upcoming-tag)
    remove_opt $1
    shift
    UPCOMING_TAG="$1"
    remove_opt $1
    ;;
  --)
    shift
    break
    ;;
esac
shift
done


if [ -z "$TAG" ]
then
  # Use latest git version tag
  TAG=$(git tag --sort=creatordate | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+' | tail -1)
  ORIGINAL_OPTS="$ORIGINAL_OPTS --since-tag $TAG"
fi

echo "Git version tag: $TAG"

cp CHANGELOG.md /tmp/CHANGELOG.md.$$
# Consolidate tag for matching changelog entries
TAG=$(echo "$TAG" | sed -e 's/-\([A-Za-z]*\)[^A-Za-z]*/-\1/' -e 's/-$//')
echo "Consolidated tag: $TAG"
sed -i -n "/^## \\[${TAG}[^]]*\\]/,\$p" CHANGELOG.md

github_changelog_generator -u CosmWasm -p cw-plus --base CHANGELOG.md $ORIGINAL_OPTS || cp /tmp/CHANGELOG.md.$$ CHANGELOG.md

if [ -n "$UPCOMING_TAG" ]
then
  # Add "upcoming" version tag
  TODAY=$(date "+%Y-%m-%d")
  sed -i "s+\[Full Changelog\](https://github.com/CosmWasm/cw-plus/compare/\(.*\)\.\.\.HEAD)+[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/$UPCOMING_TAG...HEAD)\n\n## [$UPCOMING_TAG](https://github.com/CosmWasm/cw-plus/tree/$UPCOMING_TAG) ($TODAY)\n\n[Full Changelog](https://github.com/CosmWasm/cw-plus/compare/\1...$UPCOMING_TAG)+" CHANGELOG.md
fi

rm -f /tmp/CHANGELOG.md.$$
