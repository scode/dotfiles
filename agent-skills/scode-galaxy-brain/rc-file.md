# The rc file: `~/.scode-galaxy-brainrc.md`

Read this file when `~/.scode-galaxy-brainrc.md` exists, or when the user asks to seed it. SKILL.md says what the file
is for; this file says how to interpret and maintain it.

The rc file may replace the model inventory, override profile assignments, or add natural-language routing constraints.
An inventory it supplies replaces the default inventory wholesale: omitted models are unavailable for delegation. Model
names are the ids to invoke — pass GPT names to `codex -m` and muse names to `muse exec --model` without the trailing
effort word, map Claude names to the nearest `--model` alias, and pass GLM names to `opencode run -m` prefixed with the
provider (`zai/` by default; the rc file may name another provider that serves the same model, such as a coding-plan or
router endpoint) with the effort word going to `--variant`. When a replacement inventory introduces models absent from
the built-in profiles, the rc file must assign them to profiles or describe their roles well enough to do so. Ask
instead of inventing profile assignments when that information is missing. A new family also needs an invocation
mechanism; treat it as unavailable until the rc file provides one.

Legacy rc files may still contain the old `cost`, `intelligence`, and `taste` columns. Treat known rows as an
availability inventory and preserve family/SOTA metadata, but ignore the numeric scores. Apply the built-in work
profiles after filtering them to the listed models. Unknown models still require explicit profile roles under the rule
above. Tell the user that profile overrides are now the supported way to customize routing.

To spare the user manual copy-paste, they can ask you to seed the file. On that explicit request only, write the current
model inventory and work-profile table into `~/.scode-galaxy-brainrc.md`, preceded by a note that they replace the
defaults and are meant to be edited. Never discard existing content: append when safe, and stop to ask if the file
already contains an inventory or profile table.
