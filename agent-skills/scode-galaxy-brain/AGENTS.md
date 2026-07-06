# Instructions for agents changing this skill

After changing SKILL.md, eval the change with a fresh-context sub agent before presenting the work as done. Skill text
is consumed by agents that have none of your conversation context, so your own reading of the new wording proves nothing
about how it lands cold — misrouting has repeatedly been caught only by asking a clean agent.

How to run an eval:

1. Spawn a sub agent with no prior context (a general-purpose Agent tool task works).
2. Tell it only where the relevant skill definitions live — this SKILL.md, plus any other skill involved in the scenario
   — and give it a realistic user question whose answer the change should affect. For routing changes, "if I asked you
   to use scode-galaxy-brain to do X, which model would you use?" works well.
3. Judge whether the answer reflects the intended behavior, not merely whether it quotes the new text. If it doesn't,
   revise the wording and re-run until it does.

When a scenario depends on `~/.scode-galaxy-brainrc.md`, check that the file does not already exist before creating a
temporary one, and delete it when the eval is done — it belongs to the user.
