#!/bin/bash


ORIGINAL_OPTS="$@"
OPTS=$(getopt -l "help,since-tag:,latest-tag" -o "hl" -- "$@") || exit 1

eval set -- "$OPTS"
while true
do
case $1 in
  -h|--help)
    echo -e "Usage: $0 [-h|--help] [--since-tag <tag>] [-l|--latest-tag]
-h, --help          Display help
--since-tag <tag>   Process changes since tag <tag>
-l, --latest-tag    Process changes since latest tag"
    exit 0
    ;;
--since-tag)
    shift
    TAG="$1"
    ;;
-l|--latest-tag)
    TAG=$(git tag --sort=creatordate | egrep '^v[0-9]+\.[0-9]+\.[0-9]+' | tail -1)
    ORIGINAL_OPTS=$(echo "$ORIGINAL_OPTS" | sed "s/\B$1\b/--since-tag $TAG/")
    ;;
--)
    shift
    break
    ;;
esac
shift
done

cp CHANGELOG.md /tmp/CHANGELOG.md.$$
sed -i -n "/^## \[$TAG\]/,\$p" CHANGELOG.md

github_changelog_generator -u CosmWasm -p cw-plus --base CHANGELOG.md $ORIGINAL_OPTS || cp /tmp/CHANGELOG.md.$$ CHANGELOG.md

rm -f /tmp/CHANGELOG.md.$$
