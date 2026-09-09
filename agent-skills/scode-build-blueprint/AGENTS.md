# Maintaining scode-build-blueprint

Read `SPEC.md` before changing this skill. Keep planning and execution separate, preserve the executor's bounded
authority, and update `EVALS.md` when behavior changes. Do not run model evals automatically without the user's request.

This skill participates in the layered dependency contract enforced by `tests/skill_deps.rs`. Keep its dependency
declaration, canonical loading stanzas, and registry entry in sync. Do not introduce a dependency on Galaxy Brain to
reuse its logging or orchestration policy.
