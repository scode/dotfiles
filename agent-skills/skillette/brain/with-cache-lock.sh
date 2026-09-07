#!/bin/sh
# Serialize local Git metadata changes without holding a lock across agent tool calls.
# A killed process may leave the directory behind; refusing it is safer than stealing
# a lock from a slow writer. Remote publication is protected separately by Git push.

if [ "$#" -eq 0 ]; then
    printf '%s\n' 'usage: with-cache-lock.sh command [argument ...]' >&2
    exit 2
fi

brain_cache_root=${XDG_CACHE_HOME:-$HOME/.cache}/brain
brain_lock=$brain_cache_root/scode-brain.lock
mkdir -p "$brain_cache_root" || exit 1

# mkdir is the acquisition, not the owner file. Another process may see the lock
# before the diagnostic record is written and must still treat it as occupied.
brain_attempt=0
until mkdir "$brain_lock" 2>/dev/null; do
    brain_attempt=$((brain_attempt + 1))
    if [ "$brain_attempt" -ge 5 ]; then
        printf 'brain metadata lock unavailable: %s\n' "$brain_lock" >&2
        exit 1
    fi
    sleep 1 || exit 1
done

# Install cleanup immediately after acquisition. Preserve the command's exit code:
# callers must not mistake a failed metadata operation for a successful write.
brain_unlock() {
    brain_status=$?
    trap - 0 HUP INT TERM
    if ! rm -f "$brain_lock/owner" || ! rmdir "$brain_lock"; then
        printf 'could not release brain metadata lock: %s\n' "$brain_lock" >&2
        [ "$brain_status" -ne 0 ] || brain_status=1
    fi
    exit "$brain_status"
}
trap brain_unlock 0
# On a signal, retain the lock: the child command may still be running. The owner
# record is diagnostic only, never an automatic stale-lock deletion criterion.
trap 'trap - 0; exit 129' HUP
trap 'trap - 0; exit 130' INT
trap 'trap - 0; exit 143' TERM
printf 'pid=%s\nhost=%s\n' "$$" "$(hostname)" > "$brain_lock/owner" || exit 1

"$@"
exit "$?"
