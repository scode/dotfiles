# Changing the brain skillette

Read [EVALS.md](EVALS.md) before changing this skillette. Keep its use flows and expected outcomes current when behavior
changes, and add a regression scenario when fixing a failure that a future agent could repeat. Keep the parent
[specification](../SPEC.md) consistent too.

Evals are opt-in. Do not run them merely because a file changed; run them when the user asks. Default to a cheap, fast
model available in the current harness, unless the user specifies another model or effort. Do not hardcode a model name
in the eval instructions. Record which model and effort actually ran in the run report.

`CLAUDE.md` is a relative symlink to this file. Neither this file nor `EVALS.md` belongs on the normal brain-use loading
path; they guide maintenance and requested evaluation.
