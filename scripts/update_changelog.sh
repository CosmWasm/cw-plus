#!/bin/bash


ORIGINAL_OPTS=$*
OPTS=$(getopt -l "help,since-tag:,full,token:" -o "hft" -- "$@") || exit 1

eval set -- "$OPTS"
while true
do
case $1 in
  -h|--help)
    echo -e "Usage: $0 [-h|--help] [-f|--full] [--since-tag <tag>] [-t|--token <token>]
-h, --help          Display help
-f, --full          Process changes since the beginning (by default: since latest git version tag)
--since-tag <tag>   Process changes since git version tag <tag> (by default: since latest git version tag)
--token <token>     Pass changelog github token <token>"
    exit 0
    ;;
  --since-tag)
    shift
    TAG="$1"
    ;;
  -f|--full)
    TAG="<FULL>"
    ORIGINAL_OPTS=$(echo "$ORIGINAL_OPTS" | sed "s/\\B$1\\b//")
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

rm -f /tmp/CHANGELOG.md.$$
