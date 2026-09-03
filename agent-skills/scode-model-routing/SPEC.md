# scode-model-routing specification

Dependencies: none

NOTE: This file is binding on the skill's text. It is deliberately sparse: it records only requirements that have been
stated as such, not a description of everything the skill does. Absence of an entry means the behavior is not yet
specified, not that it is unspecified on purpose. When the skill and this file disagree, that is a bug in one of them;
fix the skill or change this file in the same change, never leave them apart.

## Requirements

- The skill depends on no other skill, and its text refers to no other skill: no pointer upward or sideways into a
  consumer (a named skill or one of its files, "the gate", "the checkpoint protocol" as a thing the reader should go and
  read). Where the text needs a fact only a caller has, it names that fact as a request input and stops there.
- Loading the skill is side-effect free. Invoking it activates nothing for the session, writes nothing, and claims
  nothing about later spawns; its `SKILL.md` says so in its first paragraph and its description says it is loaded by
  other skills and inert alone. No harness-level switch turns off description-based selection: Codex's
  `allow_implicit_invocation: false` in `agents/openai.yaml` is also read by Muse Code, which then refuses to load the
  skill for the model at all (verified on Muse Code 1.0.2), and a skill that other skills load by name cannot carry it.
  The guarantee is inertness: an unsolicited load does nothing.
- Routing answers a request of the shape `SKILL.md` describes under "The routing request" with an answer of the shape
  under "The routing answer": a route (a model id with effort, `orchestrator`, `inherit`, or `no suitable route`), a
  launch mechanism (`native`, `codex exec`, `claude -p`, `muse exec`, `opencode run`), the `route exhausted` and
  `endpoint trusted` facts, the preference and cross-family divergence flags with reasons, and a one-line reason. A
  profile-table cell that reads `none` is never returned; it resolves inside routing. Routing is a pure function of the
  request and the local environment: it holds no state between requests and never decides whether to delegate or to
  escalate.
- Routing reads the model routing config file, `~/.scode-model-routing.md`, and checks CLI and credential availability
  itself when answering; those are not request inputs. If that file is absent and `~/.scode-galaxy-brainrc.md` is
  present, routing stops and tells the user to rename it rather than answering as if no config existed. Prose in this
  skill calls it "the model routing config file", never "the rc file".
- `SKILL.md` is the public surface and is kept lean, measured as what a session loads before its first delegation:
  everything needed to answer a routing request is in `SKILL.md`, and only the full inventory table, the calibration
  history, the size anchors, and config-file handling live in sidecars read on demand, each with its trigger named in
  `SKILL.md`.
- A same-named project-local skill can shadow the installed one on every harness whose loader honors project-local
  skills. That is accepted; the consumer's name check proves identity, not revision.
- The skill is meant to work on modern Linux and macOS. Commands, paths, and tools it prescribes must be available on
  both; nothing may rely on one without an equivalent for the other. No other platform is of concern, and the skill's
  text need not accommodate one.
