# Review outputs

One row per completed matrix cell: 221 reviews, zero failures. Reported counts precede assessment splits; verdict counts
refer to the resulting assessment records. Empty outputs remain visible. Model labels and exact identifiers are in
[metadata.json](metadata.json). All assessed claims are in [FINDINGS.md](FINDINGS.md); earlier one-offs are separate in
[PRIOR.md](PRIOR.md).

| Case                                  | Model / effort | Lens                    | Status    | Reported findings | Valid | Invalid | Optional | Uncertain | Out of scope | Finding records        |
| ------------------------------------- | -------------- | ----------------------- | --------- | ----------------: | ----: | ------: | -------: | --------: | -----------: | ---------------------- |
| treeward-swapped-fifo                 | Luna high      | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| treeward-swapped-fifo                 | Terra medium   | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| treeward-swapped-fifo                 | Muse high      | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| treeward-swapped-fifo                 | Luna high      | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| treeward-swapped-fifo                 | Terra medium   | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| treeward-swapped-fifo                 | Muse high      | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| treeward-swapped-fifo                 | Luna high      | docs-comments           | completed |                 2 |     1 |       0 |        1 |         0 |            0 | F001, F002             |
| treeward-swapped-fifo                 | Terra medium   | docs-comments           | completed |                 1 |     0 |       0 |        0 |         1 |            0 | F003                   |
| treeward-swapped-fifo                 | Muse high      | docs-comments           | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| treeward-swapped-fifo                 | Luna high      | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| treeward-swapped-fifo                 | Terra medium   | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| treeward-swapped-fifo                 | Muse high      | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| treeward-swapped-fifo                 | Sol high       | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| treeward-swapped-fifo                 | Luna high      | correctness-edge-inputs | completed |                 1 |     1 |       0 |        0 |         0 |            0 | F004                   |
| treeward-swapped-fifo                 | Terra medium   | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| treeward-swapped-fifo                 | Muse high      | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| treeward-swapped-fifo                 | Sol high       | correctness-edge-inputs | completed |                 1 |     1 |       0 |        0 |         0 |            0 | F005                   |
| ferricode-openai-codex-remote-auth    | Luna high      | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| ferricode-openai-codex-remote-auth    | Terra medium   | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| ferricode-openai-codex-remote-auth    | Muse high      | idiomaticity            | completed |                 1 |     0 |       1 |        0 |         0 |            0 | F006                   |
| ferricode-openai-codex-remote-auth    | Luna high      | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| ferricode-openai-codex-remote-auth    | Terra medium   | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| ferricode-openai-codex-remote-auth    | Muse high      | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| ferricode-openai-codex-remote-auth    | Luna high      | docs-comments           | completed |                 1 |     0 |       1 |        0 |         0 |            0 | F007                   |
| ferricode-openai-codex-remote-auth    | Terra medium   | docs-comments           | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| ferricode-openai-codex-remote-auth    | Muse high      | docs-comments           | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| ferricode-openai-codex-remote-auth    | Luna high      | correctness-data-flow   | completed |                 1 |     1 |       0 |        0 |         0 |            0 | F008                   |
| ferricode-openai-codex-remote-auth    | Terra medium   | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| ferricode-openai-codex-remote-auth    | Muse high      | correctness-data-flow   | completed |                 1 |     1 |       0 |        1 |         0 |            0 | F009, F010             |
| ferricode-openai-codex-remote-auth    | Sol high       | correctness-data-flow   | completed |                 1 |     1 |       0 |        0 |         0 |            0 | F014                   |
| ferricode-openai-codex-remote-auth    | Luna high      | correctness-edge-inputs | completed |                 3 |     3 |       0 |        0 |         0 |            0 | F016, F017, F018       |
| ferricode-openai-codex-remote-auth    | Terra medium   | correctness-edge-inputs | completed |                 1 |     1 |       0 |        0 |         0 |            0 | F015                   |
| ferricode-openai-codex-remote-auth    | Muse high      | correctness-edge-inputs | completed |                 2 |     2 |       0 |        1 |         0 |            0 | F011, F012, F013       |
| ferricode-openai-codex-remote-auth    | Sol high       | correctness-edge-inputs | completed |                 2 |     2 |       0 |        0 |         0 |            0 | F019, F020             |
| dotfiles-scode-chores-initial         | Luna high      | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| dotfiles-scode-chores-initial         | Terra medium   | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| dotfiles-scode-chores-initial         | Muse high      | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| dotfiles-scode-chores-initial         | Luna high      | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| dotfiles-scode-chores-initial         | Terra medium   | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| dotfiles-scode-chores-initial         | Muse high      | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| dotfiles-scode-chores-initial         | Luna high      | docs-comments           | completed |                 1 |     1 |       0 |        0 |         0 |            0 | F021                   |
| dotfiles-scode-chores-initial         | Terra medium   | docs-comments           | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| dotfiles-scode-chores-initial         | Muse high      | docs-comments           | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| dotfiles-scode-chores-initial         | Luna high      | correctness-data-flow   | completed |                 2 |     1 |       1 |        0 |         0 |            0 | F022, F028             |
| dotfiles-scode-chores-initial         | Terra medium   | correctness-data-flow   | completed |                 2 |     1 |       1 |        0 |         0 |            0 | F023, F029             |
| dotfiles-scode-chores-initial         | Muse high      | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| dotfiles-scode-chores-initial         | Sol high       | correctness-data-flow   | completed |                 2 |     1 |       0 |        0 |         0 |            1 | F026, F032             |
| dotfiles-scode-chores-initial         | Luna high      | correctness-edge-inputs | completed |                 2 |     1 |       0 |        1 |         0 |            0 | F024, F033             |
| dotfiles-scode-chores-initial         | Terra medium   | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| dotfiles-scode-chores-initial         | Muse high      | correctness-edge-inputs | completed |                 3 |     1 |       2 |        0 |         0 |            0 | F025, F030, F031       |
| dotfiles-scode-chores-initial         | Sol high       | correctness-edge-inputs | completed |                 2 |     1 |       0 |        0 |         1 |            0 | F027, F034             |
| saltybox-spec-skeleton-initial        | Luna high      | idiomaticity            | completed |                 1 |     0 |       1 |        0 |         0 |            0 | F035                   |
| saltybox-spec-skeleton-initial        | Terra medium   | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-spec-skeleton-initial        | Muse high      | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-spec-skeleton-initial        | Luna high      | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-spec-skeleton-initial        | Terra medium   | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-spec-skeleton-initial        | Muse high      | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-spec-skeleton-initial        | Luna high      | docs-comments           | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-spec-skeleton-initial        | Terra medium   | docs-comments           | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-spec-skeleton-initial        | Muse high      | docs-comments           | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-spec-skeleton-initial        | Luna high      | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-spec-skeleton-initial        | Terra medium   | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-spec-skeleton-initial        | Muse high      | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-spec-skeleton-initial        | Sol high       | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-spec-skeleton-initial        | Luna high      | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-spec-skeleton-initial        | Terra medium   | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-spec-skeleton-initial        | Muse high      | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-spec-skeleton-initial        | Sol high       | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-move-v1-crypto-initial       | Luna high      | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-move-v1-crypto-initial       | Terra medium   | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-move-v1-crypto-initial       | Muse high      | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-move-v1-crypto-initial       | Luna high      | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-move-v1-crypto-initial       | Terra medium   | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-move-v1-crypto-initial       | Muse high      | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-move-v1-crypto-initial       | Luna high      | docs-comments           | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-move-v1-crypto-initial       | Terra medium   | docs-comments           | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-move-v1-crypto-initial       | Muse high      | docs-comments           | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-move-v1-crypto-initial       | Luna high      | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-move-v1-crypto-initial       | Terra medium   | correctness-data-flow   | completed |                 1 |     0 |       1 |        0 |         0 |            0 | F036                   |
| saltybox-move-v1-crypto-initial       | Muse high      | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-move-v1-crypto-initial       | Sol high       | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-move-v1-crypto-initial       | Luna high      | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-move-v1-crypto-initial       | Terra medium   | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-move-v1-crypto-initial       | Muse high      | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-move-v1-crypto-initial       | Sol high       | correctness-edge-inputs | completed |                 1 |     0 |       1 |        0 |         0 |            0 | F037                   |
| saltybox-format-dispatch-initial      | Luna high      | idiomaticity            | completed |                 1 |     0 |       0 |        1 |         0 |            0 | F038                   |
| saltybox-format-dispatch-initial      | Terra medium   | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-format-dispatch-initial      | Muse high      | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-format-dispatch-initial      | Luna high      | ai-slop                 | completed |                 1 |     0 |       1 |        0 |         0 |            0 | F039                   |
| saltybox-format-dispatch-initial      | Terra medium   | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-format-dispatch-initial      | Muse high      | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-format-dispatch-initial      | Luna high      | docs-comments           | completed |                 1 |     1 |       0 |        0 |         0 |            0 | F040                   |
| saltybox-format-dispatch-initial      | Terra medium   | docs-comments           | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-format-dispatch-initial      | Muse high      | docs-comments           | completed |                 1 |     1 |       0 |        0 |         0 |            0 | F041                   |
| saltybox-format-dispatch-initial      | Luna high      | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-format-dispatch-initial      | Terra medium   | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-format-dispatch-initial      | Muse high      | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-format-dispatch-initial      | Sol high       | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-format-dispatch-initial      | Luna high      | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-format-dispatch-initial      | Terra medium   | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-format-dispatch-initial      | Muse high      | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-format-dispatch-initial      | Sol high       | correctness-edge-inputs | completed |                 1 |     0 |       1 |        0 |         0 |            0 | F042                   |
| saltybox-v2-engine-initial            | Luna high      | idiomaticity            | completed |                 1 |     0 |       0 |        1 |         0 |            0 | F052                   |
| saltybox-v2-engine-initial            | Terra medium   | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-engine-initial            | Muse high      | idiomaticity            | completed |                 1 |     0 |       0 |        1 |         0 |            0 | F054                   |
| saltybox-v2-engine-initial            | Luna high      | ai-slop                 | completed |                 1 |     0 |       1 |        0 |         0 |            0 | F053                   |
| saltybox-v2-engine-initial            | Terra medium   | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-engine-initial            | Muse high      | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-engine-initial            | Luna high      | docs-comments           | completed |                 2 |     2 |       0 |        0 |         0 |            0 | F043, F047             |
| saltybox-v2-engine-initial            | Terra medium   | docs-comments           | completed |                 2 |     2 |       0 |        0 |         0 |            0 | F044, F046             |
| saltybox-v2-engine-initial            | Muse high      | docs-comments           | completed |                 1 |     1 |       0 |        0 |         0 |            0 | F045                   |
| saltybox-v2-engine-initial            | Luna high      | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-engine-initial            | Terra medium   | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-engine-initial            | Muse high      | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-engine-initial            | Sol high       | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-engine-initial            | Luna high      | correctness-edge-inputs | completed |                 1 |     1 |       0 |        0 |         0 |            0 | F048                   |
| saltybox-v2-engine-initial            | Terra medium   | correctness-edge-inputs | completed |                 1 |     1 |       0 |        0 |         0 |            0 | F049                   |
| saltybox-v2-engine-initial            | Muse high      | correctness-edge-inputs | completed |                 1 |     1 |       0 |        0 |         0 |            0 | F050                   |
| saltybox-v2-engine-initial            | Sol high       | correctness-edge-inputs | completed |                 1 |     1 |       0 |        0 |         0 |            0 | F051                   |
| saltybox-v2-decrypt-initial           | Luna high      | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-decrypt-initial           | Terra medium   | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-decrypt-initial           | Muse high      | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-decrypt-initial           | Luna high      | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-decrypt-initial           | Terra medium   | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-decrypt-initial           | Muse high      | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-decrypt-initial           | Luna high      | docs-comments           | completed |                 1 |     1 |       0 |        0 |         0 |            0 | F055                   |
| saltybox-v2-decrypt-initial           | Terra medium   | docs-comments           | completed |                 1 |     1 |       0 |        0 |         0 |            0 | F056                   |
| saltybox-v2-decrypt-initial           | Muse high      | docs-comments           | completed |                 1 |     1 |       0 |        0 |         0 |            0 | F057                   |
| saltybox-v2-decrypt-initial           | Luna high      | correctness-data-flow   | completed |                 1 |     1 |       0 |        0 |         0 |            0 | F058                   |
| saltybox-v2-decrypt-initial           | Terra medium   | correctness-data-flow   | completed |                 1 |     1 |       0 |        0 |         0 |            0 | F059                   |
| saltybox-v2-decrypt-initial           | Muse high      | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-decrypt-initial           | Sol high       | correctness-data-flow   | completed |                 2 |     2 |       0 |        0 |         0 |            0 | F060, F062             |
| saltybox-v2-decrypt-initial           | Luna high      | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-decrypt-initial           | Terra medium   | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-decrypt-initial           | Muse high      | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-decrypt-initial           | Sol high       | correctness-edge-inputs | completed |                 2 |     2 |       0 |        0 |         0 |            0 | F061, F063             |
| saltybox-v2-write-gate-initial        | Luna high      | idiomaticity            | completed |                 1 |     0 |       0 |        1 |         0 |            0 | F064                   |
| saltybox-v2-write-gate-initial        | Terra medium   | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-write-gate-initial        | Muse high      | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-write-gate-initial        | Luna high      | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-write-gate-initial        | Terra medium   | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-write-gate-initial        | Muse high      | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-write-gate-initial        | Luna high      | docs-comments           | completed |                 4 |     3 |       0 |        0 |         0 |            1 | F065, F066, F067, F069 |
| saltybox-v2-write-gate-initial        | Terra medium   | docs-comments           | completed |                 2 |     2 |       0 |        0 |         0 |            0 | F068, F070             |
| saltybox-v2-write-gate-initial        | Muse high      | docs-comments           | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-write-gate-initial        | Luna high      | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-write-gate-initial        | Terra medium   | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-write-gate-initial        | Muse high      | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-write-gate-initial        | Sol high       | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-write-gate-initial        | Luna high      | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-write-gate-initial        | Terra medium   | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-write-gate-initial        | Muse high      | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-write-gate-initial        | Sol high       | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-flip-initial              | Luna high      | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-flip-initial              | Terra medium   | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-flip-initial              | Muse high      | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-flip-initial              | Luna high      | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-flip-initial              | Terra medium   | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-flip-initial              | Muse high      | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-flip-initial              | Luna high      | docs-comments           | completed |                 2 |     1 |       0 |        1 |         0 |            0 | F071, F072             |
| saltybox-v2-flip-initial              | Terra medium   | docs-comments           | completed |                 1 |     0 |       1 |        0 |         0 |            0 | F073                   |
| saltybox-v2-flip-initial              | Muse high      | docs-comments           | completed |                 1 |     1 |       0 |        0 |         0 |            0 | F075                   |
| saltybox-v2-flip-initial              | Luna high      | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-flip-initial              | Terra medium   | correctness-data-flow   | completed |                 1 |     0 |       1 |        0 |         0 |            0 | F074                   |
| saltybox-v2-flip-initial              | Muse high      | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-flip-initial              | Sol high       | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-flip-initial              | Luna high      | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-flip-initial              | Terra medium   | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-flip-initial              | Muse high      | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| saltybox-v2-flip-initial              | Sol high       | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr56-catalog-static-asset | Luna high      | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr56-catalog-static-asset | Terra medium   | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr56-catalog-static-asset | Muse high      | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr56-catalog-static-asset | Luna high      | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr56-catalog-static-asset | Terra medium   | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr56-catalog-static-asset | Muse high      | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr56-catalog-static-asset | Luna high      | docs-comments           | completed |                 2 |     1 |       0 |        1 |         0 |            0 | F076, F077             |
| stark-parts-pr56-catalog-static-asset | Terra medium   | docs-comments           | completed |                 1 |     1 |       0 |        0 |         0 |            0 | F078                   |
| stark-parts-pr56-catalog-static-asset | Muse high      | docs-comments           | completed |                 1 |     1 |       0 |        0 |         0 |            0 | F079                   |
| stark-parts-pr56-catalog-static-asset | Luna high      | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr56-catalog-static-asset | Terra medium   | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr56-catalog-static-asset | Muse high      | correctness-data-flow   | completed |                 1 |     1 |       0 |        0 |         0 |            0 | F080                   |
| stark-parts-pr56-catalog-static-asset | Sol high       | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr56-catalog-static-asset | Luna high      | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr56-catalog-static-asset | Terra medium   | correctness-edge-inputs | completed |                 1 |     1 |       0 |        0 |         1 |            0 | F081, F082             |
| stark-parts-pr56-catalog-static-asset | Muse high      | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr56-catalog-static-asset | Sol high       | correctness-edge-inputs | completed |                 1 |     1 |       0 |        0 |         0 |            0 | F083                   |
| stark-parts-pr57-catalog-only-ci      | Luna high      | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr57-catalog-only-ci      | Terra medium   | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr57-catalog-only-ci      | Muse high      | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr57-catalog-only-ci      | Luna high      | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr57-catalog-only-ci      | Terra medium   | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr57-catalog-only-ci      | Muse high      | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr57-catalog-only-ci      | Luna high      | docs-comments           | completed |                 1 |     0 |       0 |        1 |         0 |            0 | F084                   |
| stark-parts-pr57-catalog-only-ci      | Terra medium   | docs-comments           | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr57-catalog-only-ci      | Muse high      | docs-comments           | completed |                 1 |     0 |       1 |        0 |         0 |            0 | F085                   |
| stark-parts-pr57-catalog-only-ci      | Luna high      | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr57-catalog-only-ci      | Terra medium   | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr57-catalog-only-ci      | Muse high      | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr57-catalog-only-ci      | Sol high       | correctness-data-flow   | completed |                 1 |     1 |       0 |        0 |         0 |            0 | F088                   |
| stark-parts-pr57-catalog-only-ci      | Luna high      | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr57-catalog-only-ci      | Terra medium   | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr57-catalog-only-ci      | Muse high      | correctness-edge-inputs | completed |                 2 |     1 |       1 |        0 |         0 |            0 | F086, F087             |
| stark-parts-pr57-catalog-only-ci      | Sol high       | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr58-catalog-vercel-cache | Luna high      | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr58-catalog-vercel-cache | Terra medium   | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr58-catalog-vercel-cache | Muse high      | idiomaticity            | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr58-catalog-vercel-cache | Luna high      | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr58-catalog-vercel-cache | Terra medium   | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr58-catalog-vercel-cache | Muse high      | ai-slop                 | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr58-catalog-vercel-cache | Luna high      | docs-comments           | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr58-catalog-vercel-cache | Terra medium   | docs-comments           | completed |                 1 |     0 |       0 |        1 |         0 |            0 | F089                   |
| stark-parts-pr58-catalog-vercel-cache | Muse high      | docs-comments           | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr58-catalog-vercel-cache | Luna high      | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr58-catalog-vercel-cache | Terra medium   | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr58-catalog-vercel-cache | Muse high      | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr58-catalog-vercel-cache | Sol high       | correctness-data-flow   | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr58-catalog-vercel-cache | Luna high      | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr58-catalog-vercel-cache | Terra medium   | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr58-catalog-vercel-cache | Muse high      | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
| stark-parts-pr58-catalog-vercel-cache | Sol high       | correctness-edge-inputs | completed |                 0 |     0 |       0 |        0 |         0 |            0 | None                   |
