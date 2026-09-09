# Independent resource watchdog

Unattended builds and tests can exhaust RAM or disk before the executor notices. A worktree may carry a separate build
cache; tests can leave large temporary files behind. Start an independent watchdog before implementation or delegation
and keep it active while owned work runs. Checking resources occasionally in the executor's own tool loop is not enough.

Prefer a background monitor process that samples resources without a model. If the harness offers a background-monitor
equivalent with reliable alerts, use it. Otherwise launch a watchdog agent on an approved worker model/effort through
shellout, with the same recording and no-grandchildren rules as other delegates. Its only task is monitoring and alerts,
not cleanup or implementation. A model watchdog occupies a delegate slot; settle sufficient capacity during planning
rather than exceeding the global cap or disabling the watchdog to run a reviewer. Process monitors consume no model
slot.

## Sampling and alerts

Sample about every minute. Watch available disk space on each filesystem holding the repository, every worktree, scratch
and build output, the execution records, and `/tmp`. Add new paths before launching work that uses them. Deduplicate
paths on the same filesystem but keep their mapping. Watch available RAM and swap as well. Use platform tools such as
`df` plus `free` or `/proc/meminfo` on Linux, and `df`, `vm_stat`, and `sysctl` on macOS; name the metric and units. Use
available memory rather than Linux's raw free-page count. Failure to read a required metric is a monitoring failure, not
a healthy sample. Derive platform-specific thresholds explicitly when available-memory semantics differ.

Default low-resource thresholds are disk below 10% free or below 5 GiB available on any watched filesystem, and
available RAM below 10% of total. Record swap availability and pressure as diagnostic evidence; do not count swap as
spare RAM. The blueprint can set task-appropriate thresholds with reasons. Send an initial alert on threshold crossing,
another when the next sample continues falling, and a recovery alert when the affected metrics regain headroom. Include
the path/filesystem or memory metric, measured value, threshold, timestamp, and trend. An alert is a request for action,
not a line to bury in a log.

Use harness notifications where supported. Always write a private heartbeat/status artifact under this execution's
record directory, containing the monitor identity, sample time, watched paths, readings, and active alerts. This mutable
status file is separate from the executor's immutable events; only the watchdog writes it. If the harness cannot deliver
notifications, the executor must check this artifact at least once a minute and before new launches. Background long
commands and cap blocking waits accordingly; the shellout skill's longer monitoring ceiling does not override this.
Retain alert transitions and executor responses in the normal execution records, not every healthy sample forever.

## Respond, recover, and resume

On low-resource alerts, stop new workload launches. Preserve ungated results and inspect the resource consumers. Remove
only verified owned, no-longer-needed worktrees, build caches, or scratch; never sweep broad directories or other
sessions' files. If necessary, stop or wait for the owned job likely responsible, accounting for its child processes.
Restarting a failed lightweight monitor is permitted during this pause. Resume workload launches only after fresh
samples show headroom. If the machine was already below thresholds at startup, report the block instead of launching
work into known exhaustion.

Record the watchdog's exact process/job handle, start identity, status path, configuration, notification mechanism, and
last heartbeat in the working log and every handoff. Check liveness by that handle, not `pgrep -f` or a process-name
match. Missing/stale heartbeats for two sample intervals, a dead monitor, or unreadable required metrics stop new
launches and require investigation and restart. Verify a PID still identifies this monitor before signaling it. On
resume, reconcile and attach to the live monitor or restart it; never assume an old log entry proves it is running.

A shellout watchdog agent still has an explicit hard deadline. Renew monitoring by a recorded replacement before that
deadline, retaining alerts and watched paths, without exceeding the concurrency cap. A background process monitor
instead has the execution's lifecycle and must be stopped after all owned work has ended. Do not leave either form
running after completion. If the executor dies, the next executor must reconcile both the monitor and workload processes
before acting.

Verify startup from a subsequent tool call, after the launching call has returned: check the exact live handle and a
fresh heartbeat. A successful `nohup ... &` command or a heartbeat read inside that same command does not prove the
process survives harness cleanup. Prefer the harness's persistent command/session facility, keeping the monitor in its
foreground; use a detached process only after proving it survives across calls. If neither works, report monitoring
blocked rather than proceeding with an unobserved process. Check status before each workload launch and after each
bounded wait, as well as the once-a-minute checks when notifications are unavailable.

When the monitor implementation is newly constructed, use a harmless synthetic sample to check alert delivery and a
throwaway monitor termination to check death detection; never exhaust real resources for this test. Recording failure
does not waive the mandatory monitor: if its live status cannot be observed, pause new workload until monitoring is
restored.

Discovering a dead watchdog at shutdown is a coverage gap, even if its last sample was healthy. Record the last verified
heartbeat and discovery time and disclose the unmonitored interval in the final report. Restarting it after the workload
ends cannot establish that the workload was monitored; do not start a replacement merely to produce a successful stop.
