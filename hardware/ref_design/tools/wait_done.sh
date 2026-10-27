#!/bin/sh
# Wait for a long job without matching your own command line.
#
#   tools/wait_done.sh --pid 12345 [--timeout 1800]
#   tools/wait_done.sh --file .build/done [--timeout 1800]
#
# Exit 0 when the PID is gone or the marker file exists, 124 on timeout, 2 on bad arguments.
# Polls every 2 s (--every N). A job that should be waited on by marker ends with
# `touch FILE` (a failing one can `echo failed > FILE`; the file's content is printed).
# Why not `pgrep -f "print_package.py 1_case"` in `bash -c`: the bash -c command line contains
# that text, so pgrep finds the waiting shell itself and the loop only ends at its cap.
pid= file= timeout=1800 every=2
while [ $# -gt 0 ]; do
    case $1 in
        --pid) pid=$2; shift 2 ;;
        --file) file=$2; shift 2 ;;
        --timeout) timeout=$2; shift 2 ;;
        --every) every=$2; shift 2 ;;
        *) echo "usage: $0 (--pid PID | --file MARKER) [--timeout SECONDS] [--every SECONDS]" >&2; exit 2 ;;
    esac
done
[ -n "$pid$file" ] || { echo "usage: $0 (--pid PID | --file MARKER) [--timeout SECONDS]" >&2; exit 2; }
start=$(date +%s)
while :; do
    if [ -n "$pid" ] && ! kill -0 "$pid" 2>/dev/null; then echo "pid $pid finished"; exit 0; fi
    if [ -n "$file" ] && [ -e "$file" ]; then echo "marker $file found"; cat "$file" 2>/dev/null; exit 0; fi
    if [ $(( $(date +%s) - start )) -ge "$timeout" ]; then echo "timed out after ${timeout}s" >&2; exit 124; fi
    sleep "$every"
done
