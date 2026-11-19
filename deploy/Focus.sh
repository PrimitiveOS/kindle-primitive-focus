#!/bin/sh
# Name: Primitive Focus
# Author: Primitive AI
# DontUseFBInk

app=/mnt/us/focus/bin/focus-kindle
[ -x "$app" ] || exit 1
case "$0" in
    /var/tmp/primitive-focus.*/launch) work=${0%/launch} ;;
    *)
        work=$(mktemp -d /var/tmp/primitive-focus.XXXXXX) || exit 1
        if ! cp "$0" "$work/launch"; then
            rmdir "$work"
            exit 1
        fi
        cd / || exit 1
        exec /bin/sh "$work/launch"
        ;;
esac
trap 'rm -rf "$work"' EXIT
cp "$app" "$work/run" && chmod 755 "$work/run" || exit 1
"$work/run"
