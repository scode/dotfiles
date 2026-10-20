# The model routing config file: `~/.scode-model-routing.md`

Read this file when `~/.scode-model-routing.md` exists, or when the user asks to seed it. `SKILL.md` says what the file
is for (Local availability); this file says how to interpret and maintain it. Call it "the model routing config file"
when talking to the user, never "the rc file".

The config file may replace the model inventory, override profile assignments, or add natural-language routing
constraints. An inventory it supplies replaces the default inventory wholesale: omitted models are unavailable for
delegation. Model names are the ids a route names, without the trailing effort word; Claude names map to the nearest
alias. How an id is spelled for a particular CLI, and which provider serves it, are not routing's concern: a config file
that says which provider or harness to use for a model ("GLM through a coding plan") is a launch preference. Routing
does not apply it; it copies the user's words into the answer's `launch preference` field for the caller to hand to
whatever launches the route. When a replacement inventory introduces models absent from the built-in profiles, the
config file must assign them to profiles or describe their roles well enough to do so. Ask instead of inventing profile
assignments when that information is missing. A new family needs no mechanism of its own: it is `external` unless the
session's native mechanism can select it.

Legacy config files may still contain the old `cost`, `intelligence`, and `taste` columns. Treat known rows as an
availability inventory and preserve family/SOTA metadata, but ignore the numeric scores. Apply the built-in work
profiles after filtering them to the listed models. Unknown models still require explicit profile roles under the rule
above. Tell the user that profile overrides are now the supported way to customize routing.

The file replaced `~/.scode-galaxy-brainrc.md`, which is no longer read. If the old file exists and the new one does
not, stop and tell the user to rename it before answering any routing request; the format is unchanged, so a rename is
the whole migration.

To spare the user manual copy-paste, they can ask you to seed the file. On that explicit request only, write the current
model inventory (from `inventory.md`) and the work-profile table (from `SKILL.md`) into `~/.scode-model-routing.md`,
preceded by a note that they replace the defaults and are meant to be edited. Never discard existing content: append
when safe, and stop to ask if the file already contains an inventory or profile table.
