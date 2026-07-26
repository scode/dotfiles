use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, anyhow, bail, ensure};
use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

const DEFAULT_SKILL: &str = "pre-pr-review-swarm";
const DEFAULT_REPEATS: usize = 3;
const DEFAULT_SKILL_PATH: &str = "agent-skills/pre-pr-review-swarm";
const EVAL_ROOT: &str = "evals/pre-pr-review-swarm";
const RUN_ROOT: &str = "eval-runs/pre-pr-review-swarm";
const WORKTREE_ROOT: &str = "eval-worktrees";

#[derive(Debug, Args)]
pub struct EvalCommand {
    #[command(subcommand)]
    command: EvalSubcommand,
}

impl EvalCommand {
    pub fn run(self) -> Result<()> {
        let root = repo_root()?;
        let tools = ToolEnv::default();

        match self.command {
            EvalSubcommand::Run(cmd) => cmd.run_with_tools(&root, &tools),
            EvalSubcommand::Baseline(cmd) => cmd.run(&root),
            EvalSubcommand::Compare(cmd) => cmd.run_with_tools(&root, &tools),
            EvalSubcommand::Synthesize(cmd) => cmd.run_with_tools(&root, &tools),
        }
    }
}

#[derive(Debug, Subcommand)]
enum EvalSubcommand {
    /// Run a skill eval case through the selected agent backend (codex or
    /// claude).
    Run(RunCommand),
    /// Mark a completed run as a baseline.
    Baseline(BaselineCommand),
    /// Compare a candidate run against a baseline run.
    Compare(CompareCommand),
    /// Ask an agent to suggest skill changes for likely regressions.
    Synthesize(SynthesizeCommand),
}

#[derive(Debug, Args)]
struct RunCommand {
    #[arg(long, default_value = DEFAULT_SKILL)]
    skill: String,
    #[arg(long = "case")]
    case_id: String,
    #[arg(long)]
    model: String,
    #[arg(long, default_value_t = DEFAULT_REPEATS)]
    repeats: usize,
    #[arg(long)]
    label: String,
    #[arg(long)]
    skill_ref: Option<String>,
    #[arg(long)]
    skill_path: Option<PathBuf>,
    /// Restrict the run to a single reviewer charter, e.g. `test-quality`.
    ///
    /// The full swarm is expensive (one agent per reviewer charter per
    /// repeat), and
    /// charter tuning usually only needs the one reviewer being tuned. The
    /// name must match a `reviewers/<name>.md` charter in the resolved skill.
    /// Restricted and unrestricted runs measure different things, so the
    /// restriction is recorded in run.json and `compare` refuses to mix them.
    #[arg(long)]
    reviewer: Option<String>,
    /// Agent CLI the run's subject agents drive (codex|claude), default codex.
    ///
    /// Selects both the invocation shape and the accepted `--effort`
    /// vocabulary. Recorded in run.json; `compare` uses it to decide whether
    /// two runs' efforts are comparable.
    #[arg(long, value_enum, default_value_t = Backend::Codex)]
    backend: Backend,
    /// Reasoning effort for the run's agents. Vocabulary depends on
    /// `--backend`: codex accepts none|minimal|low|medium|high|xhigh, claude
    /// accepts low|medium|high|xhigh|max. Individual models may support only a
    /// subset; the preflight surfaces that before real eval spend.
    ///
    /// Both backends run with user config ignored, so this flag is the only
    /// way to control effort; when omitted the run uses the backend's built-in
    /// default. Effort changes what a run measures, so it is recorded in
    /// run.json and `compare` refuses to mix runs with different efforts.
    #[arg(long)]
    effort: Option<String>,
}

#[derive(Debug, Args)]
struct BaselineCommand {
    #[arg(long)]
    run: PathBuf,
}

#[derive(Debug, Args)]
struct CompareCommand {
    #[arg(long)]
    baseline: PathBuf,
    #[arg(long)]
    candidate: PathBuf,
    #[arg(long)]
    model: Option<String>,
    /// Agent CLI the judge and matcher agents drive (codex|claude), default
    /// codex. Independent of the backends the compared runs used, exactly like
    /// `--effort`.
    #[arg(long, value_enum, default_value_t = Backend::Codex)]
    backend: Backend,
    /// Reasoning effort for the judge and matcher agents. Vocabulary depends on
    /// `--backend`. Independent of the effort the compared runs used; when
    /// omitted the judges run at the backend's built-in default.
    #[arg(long)]
    effort: Option<String>,
}

#[derive(Debug, Args)]
struct SynthesizeCommand {
    #[arg(long)]
    comparison: PathBuf,
    #[arg(long)]
    model: Option<String>,
    /// Agent CLI the synthesis agent drives (codex|claude), default codex.
    #[arg(long, value_enum, default_value_t = Backend::Codex)]
    backend: Backend,
    /// Reasoning effort for the synthesis agent. Vocabulary depends on
    /// `--backend`. When omitted, the backend's built-in default.
    #[arg(long)]
    effort: Option<String>,
}

/// Reasoning-effort vocabulary accepted by current codex model families.
///
/// The CLI forwards this value to the selected model, whose supported subset
/// can differ. Preflight is the authoritative compatibility check: for
/// example, Luna supports `none` but not `minimal`.
const CODEX_EFFORT_LEVELS: [&str; 6] = ["none", "minimal", "low", "medium", "high", "xhigh"];

/// Reasoning-effort vocabulary the claude CLI accepts for `--effort`. Note the
/// two backends do not share a scale: codex has `minimal` at the bottom and no
/// tier above `high`, claude has no `minimal` but adds `xhigh`/`max` on top.
/// That mismatch is why cross-backend compare refuses to treat an unset effort
/// as comparable (see `CompareCommand`).
const CLAUDE_EFFORT_LEVELS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];

/// Which agent CLI a run drives its model invocations through. The two
/// backends differ in command-line shape, transcript event schema, error
/// reporting, effort vocabulary, and isolation flags; every one of those
/// concerns dispatches on this enum rather than being hard-coded to codex.
///
/// Serialized lowercase into `run.json`/`comparison.json`. `Default` is
/// `Codex` so artifacts written before the backend field existed — and every
/// pre-claude run — read back as codex runs, preserving on-disk compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
enum Backend {
    #[default]
    Codex,
    Claude,
}

/// Which behavior a run measures.
///
/// Old artifacts predate this distinction and used harness-owned fan-out, so
/// the serde default preserves their meaning instead of relabeling them as
/// full-skill runs. New unrestricted runs execute the candidate coordinator;
/// `--reviewer` keeps the cheaper direct-charter path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ExecutionMode {
    #[default]
    LegacyPanel,
    Swarm,
    Reviewer,
}

impl Backend {
    /// Stable lowercase name, used in metadata display and error text. Matches
    /// the serde/clap wire form so what a user typed and what a message prints
    /// are the same token.
    fn as_str(self) -> &'static str {
        match self {
            Backend::Codex => "codex",
            Backend::Claude => "claude",
        }
    }

    /// The reasoning-effort words this backend accepts. Kept per-backend rather
    /// than shared because the two CLIs genuinely disagree on the scale.
    fn effort_levels(self) -> &'static [&'static str] {
        match self {
            Backend::Codex => &CODEX_EFFORT_LEVELS,
            Backend::Claude => &CLAUDE_EFFORT_LEVELS,
        }
    }

    /// Reject an effort word this backend does not know, up front, like the
    /// reviewer restriction — so a typo (or a codex effort handed to claude,
    /// or vice versa) fails before any checkout or token spend rather than
    /// mid-run inside the CLI.
    fn validate_effort(self, effort: Option<&str>) -> Result<()> {
        if let Some(effort) = effort {
            ensure!(
                self.effort_levels().contains(&effort),
                "unknown effort '{effort}' for the {} backend; expected one of {}",
                self.as_str(),
                self.effort_levels().join(", ")
            );
        }
        Ok(())
    }

    /// How an unset effort is displayed in metadata and compare diagnostics.
    /// An omitted `--effort` means the backend's own built-in default, which
    /// differs by vendor and is not comparable across them.
    fn default_effort_label(self) -> &'static str {
        match self {
            Backend::Codex => "codex default",
            Backend::Claude => "claude default",
        }
    }
}

impl std::fmt::Display for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl RunCommand {
    fn run_with_tools(self, root: &Path, tools: &ToolEnv) -> Result<()> {
        ensure!(
            self.skill == DEFAULT_SKILL,
            "only {DEFAULT_SKILL} is supported in v1"
        );
        ensure!(self.repeats > 0, "--repeats must be at least 1");
        self.backend.validate_effort(self.effort.as_deref())?;

        let case = load_cases(root)?
            .remove(&self.case_id)
            .ok_or_else(|| anyhow!("unknown eval case '{}'", self.case_id))?;
        // TEMPORARY: only hand-curated cases may run while sandboxing is
        // disabled (see run_codex_json). Mined cases point at third-party
        // repos, and running an unsandboxed agent against unvetted code is
        // not acceptable. Lift this once sandboxing is resolved.
        ensure!(
            case.curation == Some(Curation::Hand),
            "case '{}' is not hand-curated; mined cases are disabled until \
             sandboxing is resolved",
            self.case_id
        );
        let target = prepare_case_checkout(root, &case, tools)?;
        let skill = resolve_skill(
            root,
            self.skill_ref.as_deref(),
            self.skill_path.as_deref(),
            tools,
        )?;
        // Validate against the resolved skill, not the working tree: with
        // --skill-ref the charter set may differ from what is checked out.
        if let Some(reviewer) = &self.reviewer {
            let charter = skill.path.join("reviewers").join(format!("{reviewer}.md"));
            ensure!(
                charter.is_file(),
                "reviewer '{reviewer}' has no charter at {}",
                charter.display()
            );
        }
        let run_id = unique_id(&format!("{}-{}", slug(&self.label), slug(&self.case_id)))?;
        let run_dir = root.join(RUN_ROOT).join(&run_id);
        fs::create_dir_all(&run_dir)?;

        let metadata = RunMetadata {
            id: run_id.clone(),
            skill: self.skill,
            label: self.label,
            model: self.model.clone(),
            repeats: self.repeats,
            case_id: case.id.clone(),
            repo: case.repo.clone(),
            subject_ref: case.subject_ref.clone(),
            subject_sha: target.subject_sha.clone(),
            base_ref: target.base_ref.clone(),
            base_sha: target.base_sha.clone(),
            curation: case.curation,
            reviewer: self.reviewer.clone(),
            execution_mode: if self.reviewer.is_some() {
                ExecutionMode::Reviewer
            } else {
                ExecutionMode::Swarm
            },
            backend: self.backend,
            effort: self.effort.clone(),
            skill_source: skill.source,
            skill_path: skill.path.display().to_string(),
            created_at: now_rfc3339()?,
        };
        write_json(&run_dir.join("run.json"), &metadata)?;

        // Charter discovery remains harness-owned only as an execution audit:
        // unrestricted runs ask the candidate skill to choose and run its own
        // panel, then account for every discoverable charter as completed or
        // deliberately skipped. `--reviewer` is the intentional exception —
        // it bypasses the coordinator so one charter can be tuned cheaply.
        let mut panel = discover_panel(&skill.path, &target.checkout)?;
        if let Some(reviewer) = &self.reviewer {
            ensure!(
                panel.iter().any(|name| name == reviewer),
                "reviewer '{reviewer}' is not spawnable for this case (base \
                 charters and condition-gated charters like spec-compliance \
                 without a SPEC.md are excluded); spawnable: {}",
                panel.join(", ")
            );
            panel.retain(|name| name == reviewer);
        }

        // Scope materialization also lives in the harness rather than in a
        // coordinator agent: every reviewer must see the identical boundary,
        // and the eval scope is always exactly base..subject.
        let scope_path = run_dir.join("scope.diff");
        write_scope_file(tools, &target, &scope_path)?;

        let spec = ModelSpec {
            backend: self.backend,
            model: &self.model,
            effort: self.effort.as_deref(),
            max_concurrent_subagents: None,
        };
        run_preflight(root, tools, spec, &run_dir)?;

        for repeat in 1..=self.repeats {
            let repeat_dir = run_dir.join(format!("repeat-{repeat}"));
            let findings = if self.reviewer.is_some() {
                let prompt_template =
                    fs::read_to_string(root.join(EVAL_ROOT).join("prompts/reviewer.md"))?;
                let schema = root
                    .join(EVAL_ROOT)
                    .join("schemas/reviewer-findings.schema.json");
                let charters_dir = skill.path.join("reviewers");
                let context = PanelContext {
                    tools,
                    spec,
                    prompt_template: &prompt_template,
                    schema: &schema,
                    scope_path: &scope_path,
                    charters_dir: &charters_dir,
                    target: &target,
                };
                run_reviewer_panel(&context, &panel, &repeat_dir)?
            } else {
                let prompt_template =
                    fs::read_to_string(root.join(EVAL_ROOT).join("prompts/swarm.md"))?;
                let schema = root
                    .join(EVAL_ROOT)
                    .join("schemas/swarm-result.schema.json");
                let context = SwarmContext {
                    tools,
                    spec,
                    prompt_template: &prompt_template,
                    schema: &schema,
                    skill_path: &skill.path,
                    scope_path: &scope_path,
                    target: &target,
                };
                run_swarm_coordinator(&context, &panel, &repeat_dir)?
            };
            write_json(&repeat_dir.join("findings.json"), &findings)?;
        }

        // Post-run verification digests the on-disk evidence (deliberately
        // re-read from artifacts, not trusted from memory) and prints the
        // inspection contract. The preflight proves the path works before
        // spend; this proves — and makes cheaply inspectable — that the real
        // agents did real work.
        let verification = verify_run(&run_dir, self.repeats)?;
        write_json(&run_dir.join("verification.json"), &verification)?;
        println!(
            "post-run verification: {} — {} repeats, {} anomalies; digest: {}",
            verification.status,
            verification.repeats.len(),
            verification.anomaly_count,
            run_dir.join("verification.json").display()
        );
        for repeat in &verification.repeats {
            if let Some(coordinator) = &repeat.coordinator {
                for anomaly in &coordinator.anomalies {
                    println!("  anomaly repeat-{} coordinator: {anomaly}", repeat.repeat);
                }
            }
            for reviewer in &repeat.reviewers {
                for anomaly in &reviewer.anomalies {
                    println!(
                        "  anomaly repeat-{} {}: {anomaly}",
                        repeat.repeat, reviewer.reviewer
                    );
                }
            }
        }
        println!(
            "REQUIRED for the launching agent: read the verification digest above, then \
             spot-check the coordinator transcript for swarm runs (repeat-N/transcript.jsonl), \
             or the reviewer transcript for --reviewer runs \
             (repeat-N/reviewers/<name>.transcript.jsonl). Confirm the agent actually reviewed \
             the scope before reporting the run's results."
        );

        println!("{}", run_dir.display());
        Ok(())
    }
}

/// Post-run evidence digest, written to `verification.json`.
///
/// Full-swarm runs expose coordinator activity and collaboration counts;
/// restricted runs expose the directly invoked reviewer. Status is `clean` or
/// `attention`; transcript-shape anomalies do not abort because the launching
/// agent must judge their severity. Collaboration mismatches are different:
/// those fail before this digest because they invalidate the run.
#[derive(Debug, Serialize, Deserialize)]
struct RunVerification {
    status: String,
    anomaly_count: usize,
    repeats: Vec<RepeatVerification>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RepeatVerification {
    repeat: usize,
    expected_reviewers: usize,
    completed_reviewers: usize,
    reviewers: Vec<ReviewerVerification>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    skipped_reviewers: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    coordinator: Option<CoordinatorVerification>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReviewerVerification {
    reviewer: String,
    findings: usize,
    #[serde(default = "one_pass")]
    passes: usize,
    /// Output tokens the agent reported for the run: codex's final
    /// `turn.completed` usage, or claude's final `result` usage. Absent when
    /// the transcript never reported completion — which is itself an anomaly.
    #[serde(skip_serializing_if = "Option::is_none")]
    output_tokens: Option<u64>,
    /// Count of agent actions the transcript recorded — codex's completed
    /// `command_execution` items, or claude's non-`StructuredOutput` tool_use
    /// blocks. The two backends count different things (see the digest
    /// functions), so this number is comparable only within a backend. Zero is
    /// not flagged — a scope-only review can legitimately run no tools — but it
    /// is recorded so the launching agent can weigh it.
    commands: usize,
    anomalies: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CoordinatorVerification {
    spawned_agents: usize,
    followups: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_tokens: Option<u64>,
    commands: usize,
    anomalies: Vec<String>,
}

/// Extract the last error message from a codex --json event stream. Errors
/// appear in two shapes — top-level `{"type":"error","message":...}` events
/// and `item.completed` items whose item type is `error` — and later events
/// supersede earlier ones (an early fallback warning is less useful than the
/// final API rejection). Returns None for streams with no error events.
fn last_error_event(stdout: &str) -> Option<String> {
    let mut last = None;
    for line in stdout.lines() {
        let Ok(event) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let message = match event.get("type").and_then(|t| t.as_str()) {
            Some("error") => event.get("message").and_then(|m| m.as_str()),
            Some("item.completed")
                if event.pointer("/item/type").and_then(|v| v.as_str()) == Some("error") =>
            {
                event.pointer("/item/message").and_then(|m| m.as_str())
            }
            _ => None,
        };
        if let Some(message) = message {
            last = Some(message.to_string());
        }
    }
    last
}

/// Digest one reviewer transcript into `(output_tokens, commands, anomalies)`,
/// dispatching to the backend that produced it. The two CLIs emit different
/// event streams and encode "no real work happened" differently, so each has
/// its own parser; only the tuple shape is shared.
fn digest_transcript(backend: Backend, path: &Path) -> (Option<u64>, usize, Vec<String>) {
    match backend {
        Backend::Codex => digest_codex_transcript(path),
        Backend::Claude => digest_claude_transcript(path),
    }
}

/// Digest a codex `--json` transcript: output tokens from the final
/// `turn.completed` event, completed command count, and anomalies for the
/// signals that indicate no real agent work happened. Unknown or non-JSON
/// lines are ignored — the transcript format carries event types we do not
/// consume.
fn digest_codex_transcript(path: &Path) -> (Option<u64>, usize, Vec<String>) {
    let mut anomalies = Vec::new();
    let Ok(raw) = fs::read_to_string(path) else {
        return (
            None,
            0,
            vec!["transcript missing or unreadable".to_string()],
        );
    };
    let mut output_tokens = None;
    let mut commands = 0;
    for line in raw.lines() {
        let Ok(event) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        match event.get("type").and_then(|t| t.as_str()) {
            Some("turn.completed") => {
                output_tokens = event
                    .pointer("/usage/output_tokens")
                    .and_then(|v| v.as_u64());
            }
            Some("item.completed")
                if event.pointer("/item/type").and_then(|v| v.as_str())
                    == Some("command_execution") =>
            {
                commands += 1;
            }
            _ => {}
        }
    }
    match output_tokens {
        None => anomalies.push("no turn.completed event in transcript".to_string()),
        Some(0) => anomalies.push("turn completed with zero output tokens".to_string()),
        Some(_) => {}
    }
    (output_tokens, commands, anomalies)
}

/// Digest a claude `stream-json` transcript: output tokens from the final
/// `result` event's usage, a count of the agent's tool calls, and anomalies
/// defined from claude's own stream shapes rather than translated from codex.
///
/// The command count deliberately counts every `tool_use` block except the
/// `StructuredOutput` tool the harness forces to carry the final answer. This
/// differs in kind from the codex digest, which counts only shell command
/// executions: claude reads and searches the checkout through dedicated tools
/// (Read, Grep, and the like), so restricting the count to `Bash` would report
/// zero for a reviewer that did substantial real work through those tools.
/// The number is therefore a proxy for agent activity, comparable only within
/// the claude backend.
///
/// Anomalies flag the claude-native signatures of a run that did nothing: no
/// terminal `result` event, a `result` that reported an error (`is_error`, or
/// an error subtype — the same `is_error || subtype != "success"` predicate
/// `run_claude_json` applies, since the two signals are independent), a
/// success `result` carrying no `structured_output` (the shape the runtime
/// path refuses as having no parseable answer), and a `result` that reported
/// missing or zero output tokens. A missing result subsumes the token check —
/// there is no usage to read — so it is reported alone, and the
/// structured-output check applies only to non-error results, since error
/// results never carry one. The digest exists so a later inspector can
/// re-derive health from disk; a transcript shape the runtime path would have
/// rejected must not digest as clean.
fn digest_claude_transcript(path: &Path) -> (Option<u64>, usize, Vec<String>) {
    let mut anomalies = Vec::new();
    let Ok(raw) = fs::read_to_string(path) else {
        return (
            None,
            0,
            vec!["transcript missing or unreadable".to_string()],
        );
    };
    let mut output_tokens = None;
    let mut commands = 0;
    let mut saw_result = false;
    let mut is_error = false;
    let mut error_subtype = None;
    let mut missing_structured = false;
    for line in raw.lines() {
        let Ok(event) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        match event.get("type").and_then(|t| t.as_str()) {
            Some("assistant") => {
                if let Some(content) = event
                    .pointer("/message/content")
                    .and_then(|value| value.as_array())
                {
                    commands += content
                        .iter()
                        .filter(|block| {
                            block.get("type").and_then(|t| t.as_str()) == Some("tool_use")
                                && block.get("name").and_then(|n| n.as_str())
                                    != Some("StructuredOutput")
                        })
                        .count();
                }
            }
            Some("result") => {
                saw_result = true;
                is_error = event
                    .get("is_error")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                error_subtype = event
                    .get("subtype")
                    .and_then(|v| v.as_str())
                    .filter(|subtype| *subtype != "success")
                    .map(|subtype| subtype.to_string());
                missing_structured = event
                    .get("structured_output")
                    .is_none_or(serde_json::Value::is_null);
                output_tokens = event
                    .pointer("/usage/output_tokens")
                    .and_then(|v| v.as_u64());
            }
            _ => {}
        }
    }
    if !saw_result {
        anomalies.push("no result event in transcript".to_string());
    } else {
        match (is_error, error_subtype) {
            (true, Some(subtype)) => {
                anomalies.push(format!(
                    "result event reported is_error (subtype '{subtype}')"
                ));
            }
            (true, None) => anomalies.push("result event reported is_error".to_string()),
            (false, Some(subtype)) => {
                anomalies.push(format!("result event reported error subtype '{subtype}'"));
            }
            (false, None) => {
                if missing_structured {
                    anomalies.push("result event carried no structured output".to_string());
                }
            }
        }
        match output_tokens {
            None => anomalies.push("result event reported no output tokens".to_string()),
            Some(0) => anomalies.push("result event reported zero output tokens".to_string()),
            Some(_) => {}
        }
    }
    (output_tokens, commands, anomalies)
}

/// Build the post-run digest from disk. Reads the same artifacts a later
/// inspector would (execution.json and per-reviewer transcripts) rather than
/// trusting in-memory state, so the digest also validates that the evidence
/// trail itself is complete.
fn verify_run(run_dir: &Path, repeats: usize) -> Result<RunVerification> {
    // Learn the backend from the on-disk run.json rather than a caller
    // argument, so the digest is driven by the same recorded fact a later
    // inspector would read — and so pre-backend run.json files digest as codex.
    let metadata: RunMetadata = read_json(&run_dir.join("run.json"))?;
    let backend = metadata.backend;
    let mut repeat_records = Vec::new();
    let mut anomaly_count = 0;
    for repeat in 1..=repeats {
        let repeat_dir = run_dir.join(format!("repeat-{repeat}"));
        let execution: ExecutionRecord = read_json(&repeat_dir.join("execution.json"))?;
        let mut reviewers = Vec::new();
        let coordinator = if let Some(coordinator) = &execution.coordinator {
            let transcript = repeat_dir.join(&coordinator.transcript);
            let (output_tokens, commands, anomalies) = digest_transcript(backend, &transcript);
            anomaly_count += anomalies.len();
            for entry in &execution.reviewers {
                reviewers.push(ReviewerVerification {
                    reviewer: entry.reviewer.clone(),
                    findings: entry.findings,
                    passes: entry.passes,
                    output_tokens: None,
                    commands: 0,
                    anomalies: Vec::new(),
                });
            }
            Some(CoordinatorVerification {
                spawned_agents: coordinator.spawned_agents,
                followups: coordinator.followups,
                output_tokens,
                commands,
                anomalies,
            })
        } else {
            for entry in &execution.reviewers {
                let transcript = repeat_dir
                    .join("reviewers")
                    .join(format!("{}.transcript.jsonl", entry.reviewer));
                let (output_tokens, commands, anomalies) = digest_transcript(backend, &transcript);
                anomaly_count += anomalies.len();
                reviewers.push(ReviewerVerification {
                    reviewer: entry.reviewer.clone(),
                    findings: entry.findings,
                    passes: entry.passes,
                    output_tokens,
                    commands,
                    anomalies,
                });
            }
            None
        };
        repeat_records.push(RepeatVerification {
            repeat,
            expected_reviewers: execution.expected,
            completed_reviewers: execution.reviewers.len(),
            reviewers,
            skipped_reviewers: execution
                .skipped_reviewers
                .iter()
                .map(|entry| entry.reviewer.clone())
                .collect(),
            coordinator,
        });
    }
    Ok(RunVerification {
        status: if anomaly_count == 0 {
            "clean"
        } else {
            "attention"
        }
        .to_string(),
        anomaly_count,
        repeats: repeat_records,
    })
}

/// Synthetic preflight scope: a four-line diff with one planted logic error
/// (`n % 2 == 1` claiming to test evenness). Deliberately trivial — any
/// functioning reviewer agent at any effort finds it — so a miss indicates a
/// broken execution path, not a hard case.
const PREFLIGHT_SCOPE: &str = "Reviewed range: preflight fixture (synthetic)\n\n\
Touched files:\nA\tsrc/even.rs\n\n\
diff --git a/src/even.rs b/src/even.rs\n\
new file mode 100644\n\
--- /dev/null\n\
+++ b/src/even.rs\n\
@@ -0,0 +1,4 @@\n\
+/// Returns true when `n` is even.\n\
+pub fn is_even(n: u32) -> bool {\n\
+    n % 2 == 1\n\
+}\n";

const PREFLIGHT_CHARTER: &str = "# preflight-reviewer\n\n\
You are a correctness reviewer. Report logic errors in the scope diff, with a file reference for each finding.\n";

/// The file a preflight finding must reference to count as evidence that the
/// agent actually read the scope rather than returning something generic.
const PREFLIGHT_PLANTED_FILE: &str = "src/even.rs";

/// Two agents rather than one: the point of the preflight is the fan-out
/// path — concurrent agent spawning with structured output — not just that
/// codex works once.
const PREFLIGHT_AGENTS: usize = 2;

/// On-disk evidence of the preflight, written to `preflight/preflight.json`
/// whether it passed or failed. This is what the invoking agent (or a human)
/// checks to confirm agent execution worked as planned before trusting the
/// run's findings.
#[derive(Debug, Serialize, Deserialize)]
struct PreflightRecord {
    status: String,
    agents: Vec<PreflightAgentRecord>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PreflightAgentRecord {
    agent: String,
    findings: usize,
    planted_issue_found: bool,
    /// Present when the agent failed outright (codex error, unparseable
    /// output) rather than running and missing the planted issue. Captured so
    /// the record stays complete on every failure path — the record is the
    /// evidence file the run output points at, and a hard failure is exactly
    /// when someone goes looking for it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Cheap end-to-end check of the agent-execution path, run before every real
/// eval: spawn two concurrent agents against a synthetic scope with a planted
/// defect and require each to find it. Exists because the failure modes it
/// guards against are silent — a schema the API rejects, a model id that
/// cannot run, or an execution path that quietly degrades — and were
/// previously discovered only after a full run's spend, or worse, not at all.
fn run_preflight(root: &Path, tools: &ToolEnv, spec: ModelSpec, run_dir: &Path) -> Result<()> {
    let dir = run_dir.join("preflight");
    fs::create_dir_all(&dir)?;
    let charter_path = dir.join("charter.md");
    let scope_path = dir.join("scope.diff");
    fs::write(&charter_path, PREFLIGHT_CHARTER)?;
    fs::write(&scope_path, PREFLIGHT_SCOPE)?;
    let template = fs::read_to_string(root.join(EVAL_ROOT).join("prompts/preflight.md"))?;
    let prompt = template
        .replace("{{charter_path}}", &charter_path.display().to_string())
        .replace("{{scope_path}}", &scope_path.display().to_string());
    let schema = root
        .join(EVAL_ROOT)
        .join("schemas/reviewer-findings.schema.json");
    let names: Vec<String> = (1..=PREFLIGHT_AGENTS)
        .map(|index| format!("preflight-{index}"))
        .collect();
    let results = std::thread::scope(|scope| {
        let handles: Vec<_> = names
            .iter()
            .map(|name| {
                let dir = &dir;
                let prompt = &prompt;
                let schema = &schema;
                scope.spawn(move || -> Result<(String, FindingsFile)> {
                    let findings_path = dir.join(format!("{name}.findings.json"));
                    let transcript_path = dir.join(format!("{name}.transcript.jsonl"));
                    run_agent(
                        tools,
                        spec,
                        dir,
                        schema,
                        &findings_path,
                        &transcript_path,
                        prompt,
                    )
                    .with_context(|| format!("preflight agent '{name}' failed"))?;
                    Ok((name.clone(), read_json(&findings_path)?))
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("preflight thread panicked"))
            .collect::<Vec<_>>()
    });
    // Write the record before propagating any agent failure: the record is
    // the evidence artifact the run output directs readers to, and a hard
    // agent failure is precisely when it gets read. Results pair with names
    // by position — the thread handles were spawned in `names` order.
    let mut agents = Vec::new();
    let mut first_error = None;
    for (name, result) in names.iter().zip(results) {
        match result {
            Ok((name, findings)) => {
                let planted = findings
                    .findings
                    .iter()
                    .any(|finding| finding.location.contains(PREFLIGHT_PLANTED_FILE));
                agents.push(PreflightAgentRecord {
                    agent: name,
                    findings: findings.findings.len(),
                    planted_issue_found: planted,
                    error: None,
                });
            }
            Err(error) => {
                agents.push(PreflightAgentRecord {
                    agent: name.clone(),
                    findings: 0,
                    planted_issue_found: false,
                    error: Some(format!("{error:#}")),
                });
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
    }
    let passed = agents.iter().all(|agent| agent.planted_issue_found);
    let record_path = dir.join("preflight.json");
    write_json(
        &record_path,
        &PreflightRecord {
            status: if passed { "passed" } else { "failed" }.to_string(),
            agents,
        },
    )?;
    if let Some(error) = first_error {
        return Err(error.context(format!("preflight record: {}", record_path.display())));
    }
    ensure!(
        passed,
        "preflight failed: an agent did not surface the planted issue, so the \
         agent-execution path cannot be trusted; see {}",
        record_path.display()
    );
    println!(
        "preflight: passed ({PREFLIGHT_AGENTS}/{PREFLIGHT_AGENTS} agents found the planted issue) — evidence: {}",
        record_path.display()
    );
    Ok(())
}

/// How many reviewer agents run concurrently within one repeat. The agents
/// are network-bound codex processes, so modest parallelism cuts wall-clock
/// substantially without saturating the machine or the API.
const REVIEWER_CONCURRENCY: usize = 6;

/// Discover reviewer charters eligible to run for this target.
///
/// This does not choose the full swarm's active panel; the candidate skill
/// does. It establishes the target-specific upper bound the coordinator must
/// account for as completed or deliberately skipped. The charter files carry
/// the exclusion markers, so the audit follows exported skill versions without
/// a separate harness manifest.
fn discover_panel(skill_path: &Path, target_root: &Path) -> Result<Vec<String>> {
    let dir = skill_path.join("reviewers");
    let mut panel = Vec::new();
    for (name, text) in reviewer_charters(skill_path)? {
        if text.contains("not spawned as a reviewer") {
            continue;
        }
        if text.contains("Only spawned when `SPEC.md` exists")
            && !target_root.join("SPEC.md").is_file()
        {
            continue;
        }
        panel.push(name);
    }
    panel.sort();
    ensure!(
        !panel.is_empty(),
        "no spawnable reviewer charters found in {}",
        dir.display()
    );
    Ok(panel)
}

/// Return every charter that can represent a reviewer, including reviewers
/// whose conditions exclude them from this target.
///
/// The coordinator may report those reviewers as deliberately skipped. Shared
/// base charters are different: they can never be reviewer executions and must
/// remain invalid output.
fn known_reviewers(skill_path: &Path) -> Result<Vec<String>> {
    let mut reviewers = reviewer_charters(skill_path)?
        .into_iter()
        .filter_map(|(name, text)| (!text.contains("not spawned as a reviewer")).then_some(name))
        .collect::<Vec<_>>();
    reviewers.sort();
    Ok(reviewers)
}

/// Read reviewer charter names and contents in deterministic order.
///
/// Discovery and alias validation share this inventory so a candidate cannot
/// get different answers merely because the filesystem returned directory
/// entries in another order.
fn reviewer_charters(skill_path: &Path) -> Result<Vec<(String, String)>> {
    let dir = skill_path.join("reviewers");
    let mut charters = Vec::new();
    for entry in fs::read_dir(&dir)
        .with_context(|| format!("no reviewers directory at {}", dir.display()))?
    {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| anyhow!("unreadable charter file name at {}", path.display()))?
            .to_string();
        charters.push((name, fs::read_to_string(&path)?));
    }
    charters.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(charters)
}

/// Resolve candidate-owned reviewer names to stable artifact keys.
///
/// Charter basenames are durable machine keys, while the first Markdown
/// heading is the skill's human-facing reviewer identity. Accepting both
/// keeps the eval adapter independent of a particular naming presentation and
/// normalizes stored attribution for offline comparisons.
fn reviewer_aliases(
    skill_path: &Path,
    known_reviewers: &[String],
) -> Result<BTreeMap<String, String>> {
    let mut aliases = BTreeMap::new();
    // Reserve every machine key before adding presentation aliases. Without
    // this first pass, a title equal to a later charter's basename is silently
    // overwritten or rejected depending on directory iteration order.
    for canonical in known_reviewers {
        ensure!(
            aliases
                .insert(canonical.clone(), canonical.clone())
                .is_none(),
            "reviewer key '{canonical}' appears more than once"
        );
    }
    for canonical in known_reviewers {
        let charter_path = skill_path.join("reviewers").join(format!("{canonical}.md"));
        let text = fs::read_to_string(&charter_path)?;
        let Some(title) = text
            .lines()
            .find_map(|line| line.trim().strip_prefix("# "))
            .map(str::trim)
            .filter(|title| !title.is_empty())
        else {
            continue;
        };
        if let Some(previous) = aliases.insert(title.to_string(), canonical.clone()) {
            ensure!(
                previous == *canonical,
                "reviewer alias '{title}' is shared by '{previous}' and '{canonical}'"
            );
        }
    }
    Ok(aliases)
}

/// Convert one coordinator-reported reviewer identity to its artifact key.
fn canonical_reviewer(aliases: &BTreeMap<String, String>, reported: &str) -> Result<String> {
    aliases
        .get(reported)
        .cloned()
        .ok_or_else(|| anyhow!("coordinator reported unknown reviewer '{reported}'"))
}

/// Materialize the review scope every reviewer receives: the base..subject
/// diff plus a touched-file summary, in one file so all agents see the exact
/// same boundary.
fn write_scope_file(tools: &ToolEnv, target: &PreparedCase, scope_path: &Path) -> Result<()> {
    let range = format!("{}..{}", target.base_sha, target.subject_sha);
    let summary = git_stdout(
        tools,
        &target.checkout,
        [
            "diff",
            "--name-status",
            &target.base_sha,
            &target.subject_sha,
        ],
    )?;
    let diff = git_stdout(
        tools,
        &target.checkout,
        [
            "diff",
            "--no-ext-diff",
            "--find-renames",
            &target.base_sha,
            &target.subject_sha,
        ],
    )?;
    ensure!(!diff.trim().is_empty(), "scope diff for {range} is empty");
    fs::write(
        scope_path,
        format!("Reviewed range: {range}\n\nTouched files:\n{summary}\n{diff}"),
    )?;
    Ok(())
}

/// Per-repeat execution evidence shared by both run modes.
///
/// A restricted run records its directly invoked reviewer. A full swarm
/// records the coordinator's claimed reviewer accounting only after the
/// transcript proves that the same number of native subagents existed.
#[derive(Debug, Serialize, Deserialize)]
struct ExecutionRecord {
    expected: usize,
    reviewers: Vec<ReviewerExecution>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    skipped_reviewers: Vec<SkippedReviewerExecution>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    coordinator: Option<CoordinatorExecution>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReviewerExecution {
    reviewer: String,
    findings: usize,
    /// Number of passes the same reviewer context completed. Direct reviewer
    /// evals always use one; swarm runs report the candidate skill's behavior.
    #[serde(default = "one_pass")]
    passes: usize,
}

/// Backward-compatible serde default for artifacts written before passes were
/// recorded.
fn one_pass() -> usize {
    1
}

/// A discoverable charter the candidate skill deliberately excluded, such as
/// a non-prose reviewer under the prose-only fast path.
#[derive(Debug, Serialize, Deserialize)]
struct SkippedReviewerExecution {
    reviewer: String,
    rationale: String,
}

/// Parent-agent collaboration evidence derived from the raw transcript rather
/// than trusted from the coordinator's structured answer.
#[derive(Debug, Serialize, Deserialize)]
struct CoordinatorExecution {
    spawned_agents: usize,
    followups: usize,
    transcript: String,
}

/// Structured adapter for a full-swarm final answer. It is intentionally
/// generic: the candidate skill decides which reviewers run and how many
/// passes they need.
#[derive(Debug, Deserialize)]
struct SwarmResult {
    findings: Vec<Finding>,
    reviewer_execution: Vec<SwarmReviewerExecution>,
}

/// One charter's coordinator-reported disposition before the harness
/// cross-checks it against discoverable charter files and collaboration
/// events.
#[derive(Debug, Deserialize)]
struct SwarmReviewerExecution {
    reviewer: String,
    status: SwarmReviewerStatus,
    passes: usize,
    rationale: String,
}

/// Whether the candidate skill ran a charter or deliberately excluded it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SwarmReviewerStatus {
    Completed,
    Skipped,
}

/// Everything a reviewer spawn needs beyond its own name: shared invocation
/// state for one run's panel, bundled so the fan-out call sites stay small.
struct PanelContext<'a> {
    tools: &'a ToolEnv,
    spec: ModelSpec<'a>,
    prompt_template: &'a str,
    schema: &'a Path,
    scope_path: &'a Path,
    charters_dir: &'a Path,
    target: &'a PreparedCase,
}

/// Inputs the candidate skill coordinator receives. The harness fixes only
/// the eval boundary and artifact format; panel choice and review behavior
/// remain inside the versioned SKILL.md under test.
struct SwarmContext<'a> {
    tools: &'a ToolEnv,
    spec: ModelSpec<'a>,
    prompt_template: &'a str,
    schema: &'a Path,
    skill_path: &'a Path,
    scope_path: &'a Path,
    target: &'a PreparedCase,
}

/// Run a directly selected reviewer panel with bounded concurrency.
///
/// Current callers use this for one `--reviewer` charter. Keeping the generic
/// panel shape preserves old artifacts and makes the isolation path usable
/// without involving the skill coordinator.
fn run_reviewer_panel(
    context: &PanelContext,
    panel: &[String],
    repeat_dir: &Path,
) -> Result<FindingsFile> {
    let reviewers_dir = repeat_dir.join("reviewers");
    fs::create_dir_all(&reviewers_dir)?;
    let mut per_reviewer: Vec<(String, FindingsFile)> = Vec::new();
    for chunk in panel.chunks(REVIEWER_CONCURRENCY) {
        let results = std::thread::scope(|scope| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|name| {
                    let reviewers_dir = &reviewers_dir;
                    scope.spawn(move || run_single_reviewer(context, name, reviewers_dir))
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().expect("reviewer thread panicked"))
                .collect::<Vec<_>>()
        });
        for result in results {
            per_reviewer.push(result?);
        }
    }

    let mut merged = Vec::new();
    let mut executions = Vec::new();
    for (name, file) in per_reviewer {
        executions.push(ReviewerExecution {
            reviewer: name.clone(),
            findings: file.findings.len(),
            passes: 1,
        });
        for mut finding in file.findings {
            finding.id = format!("{name}:{}", finding.id);
            finding.reviewers = Some(vec![name.clone()]);
            merged.push(finding);
        }
    }
    write_json(
        &repeat_dir.join("execution.json"),
        &ExecutionRecord {
            expected: panel.len(),
            reviewers: executions,
            skipped_reviewers: Vec::new(),
            coordinator: None,
        },
    )?;
    Ok(FindingsFile { findings: merged })
}

/// One reviewer agent: render its prompt, run codex against the reviewer
/// schema, and parse its findings. Runs on a worker thread during fan-out.
fn run_single_reviewer(
    context: &PanelContext,
    name: &str,
    reviewers_dir: &Path,
) -> Result<(String, FindingsFile)> {
    let charter_path = context.charters_dir.join(format!("{name}.md"));
    let prompt = context
        .prompt_template
        .replace("{{charter_path}}", &charter_path.display().to_string())
        .replace(
            "{{repo_path}}",
            &context.target.checkout.display().to_string(),
        )
        .replace("{{scope_path}}", &context.scope_path.display().to_string())
        .replace("{{base_sha}}", &context.target.base_sha)
        .replace("{{subject_sha}}", &context.target.subject_sha);
    let findings_path = reviewers_dir.join(format!("{name}.findings.json"));
    let transcript_path = reviewers_dir.join(format!("{name}.transcript.jsonl"));
    run_agent(
        context.tools,
        context.spec,
        &context.target.checkout,
        context.schema,
        &findings_path,
        &transcript_path,
        &prompt,
    )
    .with_context(|| format!("reviewer '{name}' failed"))?;
    let findings: FindingsFile = read_json(&findings_path)?;
    Ok((name.to_string(), findings))
}

/// Run the candidate skill itself as coordinator.
///
/// This path deliberately does not reproduce panel selection, continuation,
/// deduplication, or reporting in Rust. The harness supplies a fixed scope and
/// a structured-output adapter, then audits the transcript so a coordinator
/// that reviewed alone cannot pass as a swarm again.
fn run_swarm_coordinator(
    context: &SwarmContext,
    discoverable_panel: &[String],
    repeat_dir: &Path,
) -> Result<FindingsFile> {
    fs::create_dir_all(repeat_dir)?;
    let known_panel = known_reviewers(context.skill_path)?;
    let known: BTreeSet<_> = known_panel.iter().cloned().collect();
    let aliases = reviewer_aliases(context.skill_path, &known_panel)?;
    let scope_label = format!(
        "eval range {}..{}",
        context.target.base_sha, context.target.subject_sha
    );
    let prompt = context
        .prompt_template
        .replace(
            "{{skill_path}}",
            &context.skill_path.join("SKILL.md").display().to_string(),
        )
        .replace(
            "{{repo_path}}",
            &context.target.checkout.display().to_string(),
        )
        .replace("{{scope_path}}", &context.scope_path.display().to_string())
        .replace("{{scope_label}}", &scope_label);
    let result_path = repeat_dir.join("swarm-result.json");
    let transcript_path = repeat_dir.join("transcript.jsonl");
    run_agent(
        context.tools,
        context
            .spec
            .with_max_concurrent_subagents(discoverable_panel.len()),
        &context.target.checkout,
        context.schema,
        &result_path,
        &transcript_path,
        &prompt,
    )
    .context("swarm coordinator failed")?;
    let mut result: SwarmResult = read_json(&result_path)?;
    ensure_unique_finding_ids(&result.findings)
        .context("coordinator returned duplicate finding identifiers")?;

    let discoverable: BTreeSet<_> = discoverable_panel.iter().cloned().collect();
    let mut accounted = BTreeSet::new();
    let mut completed = Vec::new();
    let mut skipped = Vec::new();
    let mut completed_names = BTreeSet::new();
    for execution in result.reviewer_execution {
        let reviewer = canonical_reviewer(&aliases, &execution.reviewer)
            .with_context(|| format!("known reviewers: {}", known_panel.join(", ")))?;
        ensure!(
            accounted.insert(reviewer.clone()),
            "coordinator reported reviewer '{}' more than once",
            reviewer
        );
        match execution.status {
            SwarmReviewerStatus::Completed => {
                ensure!(
                    discoverable.contains(&reviewer),
                    "coordinator completed condition-excluded reviewer '{}'; reviewers eligible \
                     for this target: {}",
                    reviewer,
                    discoverable_panel.join(", ")
                );
                ensure!(
                    execution.passes > 0,
                    "completed reviewer '{}' reported zero passes",
                    reviewer
                );
                completed_names.insert(reviewer.clone());
                completed.push(ReviewerExecution {
                    reviewer,
                    findings: 0,
                    passes: execution.passes,
                });
            }
            SwarmReviewerStatus::Skipped => {
                ensure!(
                    execution.passes == 0,
                    "skipped reviewer '{}' reported {} passes",
                    reviewer,
                    execution.passes
                );
                ensure!(
                    !execution.rationale.trim().is_empty(),
                    "skipped reviewer '{}' has no rationale",
                    reviewer
                );
                ensure!(
                    known.contains(&reviewer),
                    "coordinator skipped unknown reviewer '{}'",
                    reviewer
                );
                skipped.push(SkippedReviewerExecution {
                    reviewer,
                    rationale: execution.rationale,
                });
            }
        }
    }
    ensure!(
        discoverable.is_subset(&accounted),
        "coordinator reviewer accounting does not match the resolved skill; missing eligible reviewers: {}",
        discoverable
            .difference(&accounted)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    );
    ensure!(
        !completed.is_empty(),
        "coordinator completed no reviewer agents"
    );

    for finding in &mut result.findings {
        let reviewers = finding.reviewers.as_ref().ok_or_else(|| {
            anyhow!(
                "coordinator finding '{}' has no reviewer attribution",
                finding.id
            )
        })?;
        ensure!(
            !reviewers.is_empty(),
            "coordinator finding '{}' has empty reviewer attribution",
            finding.id
        );
        let canonical_reviewers = reviewers
            .iter()
            .map(|reviewer| canonical_reviewer(&aliases, reviewer))
            .collect::<Result<Vec<_>>>()
            .with_context(|| format!("coordinator finding '{}'", finding.id))?;
        finding.reviewers = Some(canonical_reviewers.clone());
        for reviewer in &canonical_reviewers {
            ensure!(
                completed_names.contains(reviewer),
                "coordinator finding '{}' attributes skipped or unknown reviewer '{}'",
                finding.id,
                reviewer
            );
            let execution = completed
                .iter_mut()
                .find(|entry| &entry.reviewer == reviewer)
                .expect("completed reviewer set and records disagree");
            execution.findings += 1;
        }
    }

    let collaboration = digest_collaboration(context.spec.backend, &transcript_path)?;
    let expected_followups: usize = completed
        .iter()
        .map(|reviewer| reviewer.passes.saturating_sub(1))
        .sum();
    let record = ExecutionRecord {
        expected: completed.len(),
        reviewers: completed,
        skipped_reviewers: skipped,
        coordinator: Some(CoordinatorExecution {
            spawned_agents: collaboration.spawned_agents,
            followups: collaboration.followups,
            transcript: "transcript.jsonl".to_string(),
        }),
    };
    write_json(&repeat_dir.join("execution.json"), &record)?;
    ensure!(
        collaboration.spawned_agents == record.expected,
        "coordinator spawned {} reviewer agents but reported {} completed; transcript: {}",
        collaboration.spawned_agents,
        record.expected,
        transcript_path.display()
    );
    ensure!(
        collaboration.followups >= expected_followups,
        "coordinator recorded {} same-agent follow-ups but reported {} continuation passes; \
         transcript: {}",
        collaboration.followups,
        expected_followups,
        transcript_path.display()
    );

    Ok(FindingsFile {
        findings: result.findings,
    })
}

#[derive(Debug, Default, PartialEq, Eq)]
struct CollaborationDigest {
    spawned_agents: usize,
    followups: usize,
}

#[derive(Debug, Clone, Copy)]
enum ClaudeCollaborationCall {
    Spawn,
    Followup,
}

/// Count native subagent creation and same-agent follow-ups from the parent
/// transcript. This audit is intentionally structural: it knows the two host
/// event formats, but not the swarm's reviewer policy.
fn digest_collaboration(backend: Backend, transcript: &Path) -> Result<CollaborationDigest> {
    let raw = fs::read_to_string(transcript)
        .with_context(|| format!("failed to read {}", transcript.display()))?;
    match backend {
        Backend::Codex => {
            let mut receiver_threads = BTreeSet::new();
            let mut followups = 0;
            for line in raw.lines() {
                let Ok(event) = serde_json::from_str::<serde_json::Value>(line) else {
                    continue;
                };
                if event.get("type").and_then(|value| value.as_str()) != Some("item.completed")
                    || event.pointer("/item/type").and_then(|value| value.as_str())
                        != Some("collab_tool_call")
                    || event
                        .pointer("/item/status")
                        .and_then(|value| value.as_str())
                        != Some("completed")
                {
                    continue;
                }
                let tool = event
                    .pointer("/item/tool")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                if tool.contains("spawn_agent")
                    && let Some(receivers) = event
                        .pointer("/item/receiver_thread_ids")
                        .and_then(|value| value.as_array())
                {
                    receiver_threads.extend(
                        receivers
                            .iter()
                            .filter_map(|value| value.as_str())
                            .map(str::to_string),
                    );
                }
                if tool.contains("followup")
                    || tool.contains("send_message")
                    || tool.contains("send_input")
                {
                    followups += 1;
                }
            }
            Ok(CollaborationDigest {
                spawned_agents: receiver_threads.len(),
                followups,
            })
        }
        Backend::Claude => {
            let mut calls = BTreeMap::new();
            let mut successful_results = BTreeSet::new();
            for line in raw.lines() {
                let Ok(event) = serde_json::from_str::<serde_json::Value>(line) else {
                    continue;
                };
                let Some(content) = event
                    .pointer("/message/content")
                    .and_then(|value| value.as_array())
                else {
                    continue;
                };
                for block in content {
                    match block.get("type").and_then(|value| value.as_str()) {
                        Some("tool_use") => {
                            let Some(id) = block.get("id").and_then(|value| value.as_str()) else {
                                continue;
                            };
                            let call = match block.get("name").and_then(|value| value.as_str()) {
                                Some("Agent" | "Task") => ClaudeCollaborationCall::Spawn,
                                Some("SendMessage") => ClaudeCollaborationCall::Followup,
                                _ => continue,
                            };
                            calls.entry(id.to_string()).or_insert(call);
                        }
                        Some("tool_result")
                            if block.get("is_error").and_then(|value| value.as_bool())
                                != Some(true) =>
                        {
                            if let Some(id) =
                                block.get("tool_use_id").and_then(|value| value.as_str())
                            {
                                successful_results.insert(id.to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
            let mut digest = CollaborationDigest::default();
            for id in successful_results {
                match calls.get(&id) {
                    Some(ClaudeCollaborationCall::Spawn) => digest.spawned_agents += 1,
                    Some(ClaudeCollaborationCall::Followup) => digest.followups += 1,
                    None => {}
                }
            }
            Ok(digest)
        }
    }
}

impl BaselineCommand {
    fn run(self, root: &Path) -> Result<()> {
        let run_dir = absolutize(root, &self.run);
        let metadata: RunMetadata = read_json(&run_dir.join("run.json"))
            .with_context(|| format!("{} is not an eval run", run_dir.display()))?;
        let baseline = BaselineMarker {
            run_id: metadata.id,
            case_id: metadata.case_id,
            model: metadata.model,
            skill_source: metadata.skill_source,
            marked_at: now_rfc3339()?,
        };
        write_json(&run_dir.join("baseline.json"), &baseline)?;
        println!("{}", run_dir.join("baseline.json").display());
        Ok(())
    }
}

impl CompareCommand {
    fn run_with_tools(self, root: &Path, tools: &ToolEnv) -> Result<()> {
        let baseline_dir = absolutize(root, &self.baseline);
        let candidate_dir = absolutize(root, &self.candidate);
        ensure!(
            baseline_dir.join("baseline.json").is_file(),
            "{} has not been marked as a baseline",
            baseline_dir.display()
        );
        let baseline_meta: RunMetadata = read_json(&baseline_dir.join("run.json"))?;
        let candidate_meta: RunMetadata = read_json(&candidate_dir.join("run.json"))?;
        ensure!(
            baseline_meta.case_id == candidate_meta.case_id,
            "baseline case '{}' does not match candidate case '{}'",
            baseline_meta.case_id,
            candidate_meta.case_id
        );
        ensure!(
            baseline_meta.base_sha == candidate_meta.base_sha
                && baseline_meta.subject_sha == candidate_meta.subject_sha,
            "baseline and candidate runs must review the same resolved diff"
        );
        ensure!(
            baseline_meta.reviewer == candidate_meta.reviewer,
            "baseline reviewer restriction ({}) does not match candidate ({})",
            baseline_meta.reviewer.as_deref().unwrap_or("full panel"),
            candidate_meta.reviewer.as_deref().unwrap_or("full panel")
        );
        let baseline_mode = baseline_meta.comparison_execution_mode();
        let candidate_mode = candidate_meta.comparison_execution_mode();
        ensure!(
            baseline_mode == candidate_mode,
            "baseline execution mode ({:?}) does not match candidate ({:?})",
            baseline_mode,
            candidate_mode
        );
        // Effort comparability is backend-relative. Within one backend the
        // efforts must match exactly, as before. Across backends the A/B axis
        // this change exists to enable is allowed, but only when both runs
        // pinned an explicit effort: an unset effort means each vendor's own
        // built-in default, and those are not the same operating point, so
        // "default vs default" (or default vs pinned) would silently compare
        // incomparable configurations.
        if baseline_meta.backend == candidate_meta.backend {
            ensure!(
                baseline_meta.effort == candidate_meta.effort,
                "baseline effort ({}) does not match candidate ({})",
                baseline_meta
                    .effort
                    .as_deref()
                    .unwrap_or_else(|| baseline_meta.backend.default_effort_label()),
                candidate_meta
                    .effort
                    .as_deref()
                    .unwrap_or_else(|| candidate_meta.backend.default_effort_label())
            );
        } else {
            ensure!(
                baseline_meta.effort.is_some() && candidate_meta.effort.is_some(),
                "cross-backend compare ({} baseline vs {} candidate) requires both runs to \
                 pin --effort explicitly, because an unset effort is each backend's own \
                 built-in default and those defaults are not comparable across backends",
                baseline_meta.backend,
                candidate_meta.backend
            );
        }
        self.backend.validate_effort(self.effort.as_deref())?;
        // The judge model defaults to the candidate run's model, which only
        // makes sense when the judge backend matches the backend that model id
        // belongs to. Without this check, comparing a claude candidate with the
        // default codex judge backend hands a claude model id to codex and
        // fails only at judge spend time, after checkout — the opposite of the
        // fail-before-spend rule every other misconfiguration here follows.
        if self.model.is_none() {
            ensure!(
                self.backend == candidate_meta.backend,
                "judge backend ({}) does not match the candidate run's backend ({}), so the \
                 candidate model '{}' cannot be the default judge model; pass --model explicitly",
                self.backend,
                candidate_meta.backend,
                candidate_meta.model
            );
        }
        let model = self.model.unwrap_or_else(|| candidate_meta.model.clone());
        let spec = ModelSpec {
            backend: self.backend,
            model: &model,
            effort: self.effort.as_deref(),
            max_concurrent_subagents: None,
        };
        let comparison_id = unique_id(&format!(
            "compare-{}-{}",
            slug(&baseline_meta.case_id),
            slug(&candidate_meta.label)
        ))?;
        let comparison_dir = root.join(RUN_ROOT).join(&comparison_id);
        fs::create_dir_all(&comparison_dir)?;

        let baseline_findings = collect_run_findings(&baseline_dir)?;
        let candidate_findings = collect_run_findings(&candidate_dir)?;
        let target = prepare_case_checkout(
            root,
            &EvalCase {
                id: baseline_meta.case_id.clone(),
                repo: baseline_meta.repo.clone(),
                subject_ref: baseline_meta.subject_sha.clone(),
                base_ref: Some(baseline_meta.base_sha.clone()),
                curation: baseline_meta.curation,
            },
            tools,
        )?;

        let judgments = judge_findings(
            root,
            tools,
            spec,
            &target,
            &comparison_dir,
            "baseline",
            &baseline_findings,
        )?;
        let candidate_judgments = judge_findings(
            root,
            tools,
            spec,
            &target,
            &comparison_dir,
            "candidate",
            &candidate_findings,
        )?;
        let matches = match_findings(
            root,
            tools,
            spec,
            &target,
            &comparison_dir,
            &baseline_findings,
            &candidate_findings,
        )?;
        let comparison = build_comparison(
            &baseline_meta,
            &candidate_meta,
            baseline_findings,
            candidate_findings,
            judgments,
            candidate_judgments,
            matches,
        );
        write_json(&comparison_dir.join("comparison.json"), &comparison)?;
        println!("{}", comparison_dir.join("comparison.json").display());
        Ok(())
    }
}

impl SynthesizeCommand {
    fn run_with_tools(self, root: &Path, tools: &ToolEnv) -> Result<()> {
        let comparison_path = absolutize(root, &self.comparison);
        let comparison: ComparisonFile = read_json(&comparison_path)?;
        // Same defaulting coherence rule as compare: the candidate model id is
        // only a usable default when the synthesis backend matches the backend
        // that produced it, and comparison.json records exactly that.
        if self.model.is_none() && comparison.candidate_model.is_some() {
            ensure!(
                self.backend == comparison.candidate_backend,
                "synthesis backend ({}) does not match the comparison's candidate backend ({}), \
                 so the candidate model '{}' cannot be the default synthesis model; pass --model \
                 explicitly",
                self.backend,
                comparison.candidate_backend,
                comparison.candidate_model.as_deref().unwrap_or_default()
            );
        }
        let model = self
            .model
            .or_else(|| comparison.candidate_model.clone())
            .ok_or_else(|| anyhow!("--model is required when comparison has no candidate model"))?;
        self.backend.validate_effort(self.effort.as_deref())?;
        let prompt_template =
            fs::read_to_string(root.join(EVAL_ROOT).join("prompts/synthesize.md"))?;
        let schema = root.join(EVAL_ROOT).join("schemas/suggestions.schema.json");
        let out_dir = comparison_path
            .parent()
            .ok_or_else(|| anyhow!("comparison path has no parent"))?;
        let input = serde_json::to_string_pretty(&comparison)?;
        let prompt = format!("{prompt_template}\n\nComparison input:\n\n```json\n{input}\n```");
        let suggestions_path = out_dir.join("suggestions.json");
        let transcript_path = out_dir.join("synthesis-transcript.jsonl");
        run_agent(
            tools,
            ModelSpec {
                backend: self.backend,
                model: &model,
                effort: self.effort.as_deref(),
                max_concurrent_subagents: None,
            },
            root,
            &schema,
            &suggestions_path,
            &transcript_path,
            &prompt,
        )?;
        let suggestions: SuggestionsFile = read_json(&suggestions_path)?;
        write_json(&suggestions_path, &suggestions)?;
        println!("{}", suggestions_path.display());
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct CasesFile {
    cases: Vec<EvalCase>,
}

/// Provenance of an eval case: hand-curated versus mass-mined from upstream
/// open source PRs. Metadata only — the harness treats both kinds identically.
/// The flag exists so history preserves which cases were individually vetted
/// by a human and which came out of an agent mining pipeline with only a
/// shortlist-level skim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum Curation {
    Hand,
    Mined,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct EvalCase {
    id: String,
    repo: String,
    subject_ref: String,
    #[serde(default)]
    base_ref: Option<String>,
    #[serde(default)]
    curation: Option<Curation>,
}

#[derive(Debug)]
struct PreparedCase {
    checkout: PathBuf,
    subject_sha: String,
    base_ref: String,
    base_sha: String,
}

#[derive(Debug)]
struct ToolEnv {
    git: PathBuf,
    codex: PathBuf,
    claude: PathBuf,
}

impl Default for ToolEnv {
    fn default() -> Self {
        Self {
            git: PathBuf::from("git"),
            codex: PathBuf::from("codex"),
            claude: PathBuf::from("claude"),
        }
    }
}

#[derive(Debug)]
struct ResolvedSkill {
    path: PathBuf,
    source: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RunMetadata {
    id: String,
    skill: String,
    label: String,
    model: String,
    repeats: usize,
    case_id: String,
    repo: String,
    subject_ref: String,
    subject_sha: String,
    base_ref: String,
    base_sha: String,
    /// Copied from the case so run artifacts preserve provenance even if the
    /// case is later edited or removed. Absent in run.json files predating the
    /// field.
    #[serde(default)]
    curation: Option<Curation>,
    /// Present when the run was restricted to a single reviewer charter via
    /// `--reviewer`. A restricted run's findings are not comparable to a full
    /// panel's, so `compare` requires this field to match between baseline and
    /// candidate. Absent in run.json files predating the field, which reads
    /// back as an unrestricted run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reviewer: Option<String>,
    /// Whether this run exercised the skill coordinator or a direct reviewer.
    /// The legacy default keeps older harness-fan-out artifacts honest.
    #[serde(default)]
    execution_mode: ExecutionMode,
    /// Agent CLI that ran this run's subject agents. `#[serde(default)]` makes
    /// run.json files written before the field existed — every codex-only run —
    /// read back as `Backend::Codex`, so old artifacts still parse and compare.
    #[serde(default)]
    backend: Backend,
    /// Reasoning effort the run's agents used, when set via `--effort`. Like
    /// the reviewer restriction, effort changes what the run measures, so
    /// `compare` requires it to match between baseline and candidate. Absent
    /// both in run.json files predating the field and in runs that used
    /// codex's built-in default; the two are indistinguishable and comparable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    effort: Option<String>,
    skill_source: String,
    skill_path: String,
    created_at: String,
}

impl RunMetadata {
    /// Return the workflow this artifact actually measured for comparison.
    ///
    /// Artifacts written before `execution_mode` existed deserialize as
    /// `LegacyPanel`. Restricted runs from that era already used the same
    /// direct-charter path as today's `Reviewer` mode, so treating them as a
    /// legacy panel would invalidate compatible baselines. Only old
    /// unrestricted runs used harness-owned panel fan-out.
    fn comparison_execution_mode(&self) -> ExecutionMode {
        if self.execution_mode == ExecutionMode::LegacyPanel && self.reviewer.is_some() {
            ExecutionMode::Reviewer
        } else {
            self.execution_mode
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct BaselineMarker {
    run_id: String,
    case_id: String,
    model: String,
    skill_source: String,
    marked_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FindingsFile {
    findings: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Finding {
    id: String,
    category: String,
    summary: String,
    location: String,
    rationale: String,
    /// Reviewer charters that surfaced this finding, as reported by the
    /// skill's coordinator. Optional for compatibility with runs produced
    /// before attribution existed. The field must be declared here even
    /// though nothing in the harness consumes it: the run command
    /// re-serializes findings.json through this struct, so an undeclared
    /// field would be silently stripped from the stored artifact.
    ///
    /// More than one entry means the coordinator merged same-location
    /// findings from several reviewers — that is the signal that lets a
    /// later analysis measure per-reviewer marginal contribution (for
    /// example, whether a generalist reviewer finds anything its lensed
    /// siblings missed) from a single full-panel run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reviewers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repeat: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JudgmentsFile {
    judgments: Vec<Judgment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Judgment {
    finding_id: String,
    classification: Classification,
    rationale: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Classification {
    Good,
    Incorrect,
    Indeterminate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MatchFile {
    matches: Vec<FindingMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FindingMatch {
    baseline_finding_id: String,
    candidate_finding_id: String,
    rationale: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ComparisonFile {
    baseline_run: String,
    candidate_run: String,
    case_id: String,
    candidate_model: Option<String>,
    /// Backends of the compared runs, so `comparison.json` is self-describing —
    /// a cross-backend comparison records which vendor produced each side.
    /// `#[serde(default)]` keeps pre-backend comparison.json files parseable
    /// (both fields read back as codex).
    #[serde(default)]
    baseline_backend: Backend,
    #[serde(default)]
    candidate_backend: Backend,
    matches: Vec<FindingMatch>,
    likely_regressions: Vec<LikelyRegression>,
    nondeterminism_notes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LikelyRegression {
    baseline_finding_id: String,
    summary: String,
    rationale: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SuggestionsFile {
    suggestions: Vec<Suggestion>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Suggestion {
    summary: String,
    rationale: String,
    target: String,
}

fn repo_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("failed to locate repository root with git")?;
    ensure!(
        output.status.success(),
        "git rev-parse failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(PathBuf::from(String::from_utf8(output.stdout)?.trim()))
}

fn load_cases(root: &Path) -> Result<BTreeMap<String, EvalCase>> {
    let path = root.join(EVAL_ROOT).join("cases.toml");
    let file: CasesFile = toml::from_str(&fs::read_to_string(&path)?)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    let mut cases = BTreeMap::new();
    for case in file.cases {
        let id = case.id.clone();
        ensure!(
            cases.insert(id.clone(), case).is_none(),
            "duplicate eval case id '{}'",
            id
        );
    }
    Ok(cases)
}

fn prepare_case_checkout(root: &Path, case: &EvalCase, tools: &ToolEnv) -> Result<PreparedCase> {
    let checkout = root
        .join(WORKTREE_ROOT)
        .join("repos")
        .join(slug(&case.repo));
    if checkout.join(".git").exists() {
        run_checked(
            Command::new(&tools.git)
                .arg("-C")
                .arg(&checkout)
                .arg("fetch")
                .arg("--tags")
                .arg("origin"),
        )?;
    } else {
        let parent = checkout
            .parent()
            .ok_or_else(|| anyhow!("checkout path has no parent"))?;
        fs::create_dir_all(parent)?;
        let url = format!("https://github.com/{}.git", case.repo);
        run_checked(
            Command::new(&tools.git)
                .arg("clone")
                .arg("--quiet")
                .arg(url)
                .arg(&checkout),
        )?;
    }

    let subject_sha = resolve_case_ref(tools, &checkout, &case.subject_ref)?;
    run_checked(
        Command::new(&tools.git)
            .arg("-C")
            .arg(&checkout)
            .args(["checkout", "--quiet"])
            .arg(&subject_sha),
    )?;
    let (base_ref, base_sha) = if let Some(base_ref) = &case.base_ref {
        let sha = resolve_case_ref(tools, &checkout, base_ref)?;
        (base_ref.clone(), sha)
    } else {
        let parents = git_stdout(tools, &checkout, ["show", "-s", "--format=%P", "HEAD"])?;
        let parent_shas: Vec<_> = parents.split_whitespace().collect();
        ensure!(
            parent_shas.len() == 1,
            "case '{}' subject ref resolves to a merge/root commit; set base_ref explicitly",
            case.id
        );
        (parent_shas[0].to_string(), parent_shas[0].to_string())
    };

    Ok(PreparedCase {
        checkout,
        subject_sha,
        base_ref,
        base_sha,
    })
}

fn resolve_skill(
    root: &Path,
    skill_ref: Option<&str>,
    skill_path: Option<&Path>,
    tools: &ToolEnv,
) -> Result<ResolvedSkill> {
    let (path, source) = match (skill_ref, skill_path) {
        (Some(_), Some(_)) => bail!("use only one of --skill-ref or --skill-path"),
        (None, Some(path)) => (absolutize(root, path), "path".to_string()),
        (Some(reference), None) => {
            let out = root
                .join(WORKTREE_ROOT)
                .join("skills")
                .join(unique_id(&slug(reference))?);
            export_skill_ref(root, reference, &out, tools)?;
            (out, format!("git:{reference}"))
        }
        (None, None) => (root.join(DEFAULT_SKILL_PATH), "working-tree".to_string()),
    };
    ensure!(
        path.join("SKILL.md").is_file(),
        "{} does not look like a skill directory",
        path.display()
    );
    Ok(ResolvedSkill { path, source })
}

fn export_skill_ref(root: &Path, reference: &str, out: &Path, tools: &ToolEnv) -> Result<()> {
    if out.exists() {
        fs::remove_dir_all(out)?;
    }
    fs::create_dir_all(out)?;
    let archive = Command::new(&tools.git)
        .arg("-C")
        .arg(root)
        .arg("archive")
        .arg(reference)
        .arg(DEFAULT_SKILL_PATH)
        .output()
        .with_context(|| format!("failed to archive skill at ref {reference}"))?;
    ensure!(
        archive.status.success(),
        "git archive failed: {}",
        String::from_utf8_lossy(&archive.stderr)
    );
    let mut tar = Command::new("tar")
        .arg("-x")
        .arg("-C")
        .arg(out)
        .arg("--strip-components=2")
        .stdin(Stdio::piped())
        .spawn()
        .context("failed to start tar")?;
    std::io::copy(
        &mut archive.stdout.as_slice(),
        tar.stdin
            .as_mut()
            .ok_or_else(|| anyhow!("tar stdin unavailable"))?,
    )?;
    drop(tar.stdin.take());
    let status = tar.wait()?;
    ensure!(
        status.success(),
        "tar failed while extracting {DEFAULT_SKILL_PATH}/"
    );
    ensure!(
        out.join("SKILL.md").is_file(),
        "ref {reference} did not contain {DEFAULT_SKILL_PATH}"
    );
    Ok(())
}

/// Backend, model id, and execution overrides for one agent invocation.
///
/// Bundled so a call site cannot choose a model without deciding its backend
/// and validated effort. Coordinator capacity belongs here too: it changes
/// native execution plumbing without changing the candidate's review policy.
#[derive(Clone, Copy)]
struct ModelSpec<'a> {
    backend: Backend,
    model: &'a str,
    effort: Option<&'a str>,
    max_concurrent_subagents: Option<usize>,
}

impl ModelSpec<'_> {
    /// Supply native coordinator capacity derived from the candidate panel.
    fn with_max_concurrent_subagents(self, max: usize) -> Self {
        Self {
            max_concurrent_subagents: Some(max),
            ..self
        }
    }
}

/// Run one agent invocation through the backend the spec selects, writing the
/// raw event stream to `transcript` and the agent's final structured answer to
/// `output_last_message` (the file downstream `read_json` consumers parse).
/// This is the single seam every model call funnels through — preflight,
/// reviewers, judges, matcher, synthesis — so backend choice is decided here
/// and nowhere else.
fn run_agent(
    tools: &ToolEnv,
    spec: ModelSpec,
    cwd: &Path,
    schema: &Path,
    output_last_message: &Path,
    transcript: &Path,
    prompt: &str,
) -> Result<()> {
    let run = match spec.backend {
        Backend::Codex => run_codex_json,
        Backend::Claude => run_claude_json,
    };
    run(
        tools,
        spec,
        cwd,
        schema,
        output_last_message,
        transcript,
        prompt,
    )
}

fn run_codex_json(
    tools: &ToolEnv,
    spec: ModelSpec,
    cwd: &Path,
    schema: &Path,
    output_last_message: &Path,
    transcript: &Path,
    prompt: &str,
) -> Result<()> {
    let mut command = Command::new(&tools.codex);
    command
        .arg("exec")
        .arg("--json")
        .arg("--ephemeral")
        .arg("--ignore-user-config")
        .arg("--ignore-rules")
        // TEMPORARY: sandboxing is disabled until the host sandbox situation
        // is resolved. Codex's Linux sandbox wraps every shell command in
        // bubblewrap, which cannot start on hosts that restrict unprivileged
        // user namespaces (Ubuntu's apparmor_restrict_unprivileged_userns)
        // unless a profiled system bwrap is installed. The failure mode is
        // nasty: every agent command exits 1 before running, the agent
        // silently falls back to web/MCP lookups, and the eval measures
        // nothing. Until sandboxing works here, run unsandboxed and restrict
        // the case list to hand-curated repos (see RunCommand). Restore
        // `-s read-only` when re-enabling.
        .arg("--dangerously-bypass-approvals-and-sandbox")
        .arg("-m")
        .arg(spec.model);
    // Effort must ride on the command line: --ignore-user-config strips the
    // config file where model_reasoning_effort would normally live, so
    // omitting the flag here means codex's built-in default, not the user's.
    if let Some(effort) = spec.effort {
        command
            .arg("-c")
            .arg(format!("model_reasoning_effort={effort}"));
    }
    if let Some(max_concurrent_subagents) = spec.max_concurrent_subagents {
        command.arg("-c").arg(format!(
            "agents.max_concurrent_threads_per_session={max_concurrent_subagents}"
        ));
    }
    let mut child = command
        .arg("-C")
        .arg(cwd)
        .arg("--output-schema")
        .arg(schema)
        .arg("-o")
        .arg(output_last_message)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to start codex exec")?;
    // BrokenPipe means the child died before draining stdin (a flag rejected
    // at parse time, for example). Tolerate it so wait_with_output still runs
    // and the failure path below reports the real cause with a transcript,
    // instead of a bare "Broken pipe" with no evidence trail. The tolerance is
    // for the failure path only: a child that closed stdin early but then
    // exits 0 ran on a truncated prompt, and trusting its output would be
    // silent eval invalidation — the success path below refuses that shape.
    let mut stdin_broken = false;
    match child
        .stdin
        .as_mut()
        .ok_or_else(|| anyhow!("codex stdin unavailable"))?
        .write_all(prompt.as_bytes())
    {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => {
            stdin_broken = true;
        }
        Err(error) => {
            // Kill and reap before propagating: Child's drop neither kills
            // nor reaps, so a bare return would leave an unsandboxed agent
            // running unsupervised with no transcript on disk. The write
            // error is the real cause; kill/wait failures add nothing.
            let _ = child.kill();
            let _ = child.wait();
            return Err(error).context("failed to write prompt to codex stdin");
        }
    }
    let output = child
        .wait_with_output()
        .context("failed to run codex exec")?;
    fs::write(transcript, &output.stdout)?;
    if !output.status.success() {
        // Codex reports failures as events on the --json stdout stream, not
        // stderr, so stderr alone is usually empty and useless. Surface the
        // last error event and the transcript path so the actual cause (a
        // rejected schema, an unknown model) reaches the command output
        // instead of requiring a manual transcript dig.
        let stdout = String::from_utf8_lossy(&output.stdout);
        let event = last_error_event(&stdout)
            .map(|message| format!("; last error event: {message}"))
            .unwrap_or_default();
        bail!(
            "codex exec failed: {}{event}; transcript: {}",
            String::from_utf8_lossy(&output.stderr).trim(),
            transcript.display()
        );
    }
    ensure!(
        !stdin_broken,
        "codex exited 0 but closed stdin before reading the full prompt; its output cannot \
         be trusted; transcript: {}",
        transcript.display()
    );
    ensure!(
        output_last_message.is_file(),
        "codex did not write {}",
        output_last_message.display()
    );
    Ok(())
}

/// Run one claude agent, saving the `stream-json` event stream as the
/// transcript and extracting the final message into `output_last_message` so
/// downstream consumers stay identical to the codex path.
///
/// Flag rationale (verified 2026-07-18 on this host; do not re-probe):
///
/// - `--safe-mode` is the isolation analog of codex's
///   `--ignore-user-config --ignore-rules`. A probe with plain
///   `claude -p --model haiku` confirmed the user's `~/.claude/CLAUDE.md`
///   reaches the agent by default and that a `CLAUDE.md` in the cwd does too;
///   the same probe with `--safe-mode` confirmed neither does. A second probe
///   covered the executable channels, which matter more than prompt
///   contamination because the child runs unsandboxed in the target checkout:
///   a planted cwd `.claude/settings.json` (SessionStart and PreToolUse hooks)
///   and cwd `.mcp.json` server all executed in default mode and none of them
///   executed under `--safe-mode`. Both arms of the executable-channel probe
///   included `--dangerously-skip-permissions`, so the exact flag combination
///   shipped here is what was verified — skip-permissions does not re-enable
///   project config that `--safe-mode` disabled. Auth still uses OAuth normally under
///   `--safe-mode` (no `ANTHROPIC_API_KEY` in the environment), which is why
///   `--bare` is deliberately NOT used: `--bare` forces API-key-only auth and
///   breaks subscription-authenticated hosts.
/// - `--dangerously-skip-permissions` is the sandbox-bypass analog of codex's
///   `--dangerously-bypass-approvals-and-sandbox`. The same caveat applies:
///   agent commands run unsandboxed, so the case list must stay hand-curated.
/// - There is no `-o` last-message flag and no `-C` working-directory flag:
///   the final answer is extracted from the terminal `result` event, and the
///   cwd is set on the child process.
/// - `CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS` raises print mode's 600s
///   background-wait ceiling — which kills legitimate long agent work — to one
///   hour, deliberately finite rather than 0 (unlimited): the harness has no
///   watchdog of its own, so unlimited would turn a wedged child into an
///   infinite hang of the whole eval. The ceiling's exit-0-with-diagnostic
///   shape is refused loudly by the zero-exit checks in the function body.
fn run_claude_json(
    tools: &ToolEnv,
    spec: ModelSpec,
    cwd: &Path,
    schema: &Path,
    output_last_message: &Path,
    transcript: &Path,
    prompt: &str,
) -> Result<()> {
    // The claude CLI wants the schema as a literal string argument, not a
    // path, so the harness reads it and hands over the contents.
    let schema_contents = claude_schema_contents(schema)?;
    let mut command = Command::new(&tools.claude);
    command
        .arg("-p")
        .arg("--safe-mode")
        .arg("--dangerously-skip-permissions")
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--no-session-persistence")
        .arg("--json-schema")
        .arg(&schema_contents)
        .arg("--model")
        .arg(spec.model);
    if let Some(effort) = spec.effort {
        command.arg("--effort").arg(effort);
    }
    let mut child = command
        // No `-C` flag: the target checkout is the child's working directory.
        .current_dir(cwd)
        // One hour, not 0 (unlimited): the default 600s ceiling kills
        // legitimate long agent work, but unlimited would leave a wedged child
        // as an infinite hang — the harness has no watchdog of its own, and
        // one hung reviewer wedges the whole unattended eval. A ceiling kill
        // exits 0 with a diagnostic instead of a result, a shape the zero-exit
        // checks below refuse loudly, so a finite ceiling is safe.
        .env("CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS", "3600000")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to start claude")?;
    // A child that dies before draining stdin — most plausibly an installed
    // CLI version rejecting one of the newer flags at argument parse time —
    // surfaces here as BrokenPipe. Tolerate exactly that error so
    // wait_with_output still runs: the transcript still gets written and the
    // failure path below reports the real cause instead of a bare
    // "Broken pipe" with no evidence trail. Tolerated for the failure path
    // only: a child that closed stdin early but exits 0 ran on a truncated
    // prompt — even a schema-valid answer from it is untrustworthy, so the
    // success path below refuses that shape.
    let mut stdin_broken = false;
    match child
        .stdin
        .as_mut()
        .ok_or_else(|| anyhow!("claude stdin unavailable"))?
        .write_all(prompt.as_bytes())
    {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => {
            stdin_broken = true;
        }
        Err(error) => {
            // Kill and reap before propagating: Child's drop neither kills
            // nor reaps, so a bare return would leave an unsandboxed agent
            // running unsupervised with no transcript on disk. The write
            // error is the real cause; kill/wait failures add nothing.
            let _ = child.kill();
            let _ = child.wait();
            return Err(error).context("failed to write prompt to claude stdin");
        }
    }
    let output = child.wait_with_output().context("failed to run claude")?;
    fs::write(transcript, &output.stdout)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        // Like codex, claude reports failures inside the event stream, not on
        // stderr; surface the final result event's cause and the transcript
        // path so the actual reason reaches the command output.
        let cause = claude_error_cause(&stdout)
            .map(|message| format!("; cause: {message}"))
            .unwrap_or_default();
        bail!(
            "claude failed: {}{cause}; transcript: {}",
            String::from_utf8_lossy(&output.stderr).trim(),
            transcript.display()
        );
    }
    ensure!(
        !stdin_broken,
        "claude exited 0 but closed stdin before reading the full prompt; its output cannot \
         be trusted; transcript: {}",
        transcript.display()
    );
    // A zero exit is not success by itself: claude can exit 0 while the
    // terminal `result` event reports an error (the background-wait diagnostic
    // documented above is one such shape). Check the event before trusting the
    // payload, and fail loudly naming the transcript rather than write a bogus
    // or empty last-message file.
    let result = last_result_event(&stdout).ok_or_else(|| {
        anyhow!(
            "claude exited 0 but emitted no result event; transcript: {}",
            transcript.display()
        )
    })?;
    let is_error = result
        .get("is_error")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let subtype = result.get("subtype").and_then(|value| value.as_str());
    if is_error || subtype.is_some_and(|subtype| subtype != "success") {
        let cause = claude_error_cause(&stdout)
            .map(|message| format!(": {message}"))
            .unwrap_or_default();
        bail!(
            "claude exited 0 but its result event reported an error{cause}; transcript: {}",
            transcript.display()
        );
    }
    let last_message = claude_last_message(&result).ok_or_else(|| {
        anyhow!(
            "claude exited 0 but produced no structured output; transcript: {}",
            transcript.display()
        )
    })?;
    fs::write(output_last_message, last_message)?;
    Ok(())
}

/// Schema contents as claude's `--json-schema` will accept them. The checked-in
/// schema files declare `"$schema": ".../draft/2020-12/schema"`, which codex
/// accepts but claude's validator rejects outright ("no schema with key or ref
/// ..." — it cannot resolve the meta-schema reference). Discovered on the first
/// real claude smoke run, 2026-07-18. The files stay self-describing on disk;
/// the key is stripped only from what is handed to claude.
fn claude_schema_contents(schema: &Path) -> Result<String> {
    let raw = fs::read_to_string(schema)
        .with_context(|| format!("failed to read schema {}", schema.display()))?;
    let mut value: serde_json::Value = serde_json::from_str(&raw)
        .with_context(|| format!("schema {} is not valid JSON", schema.display()))?;
    if let Some(object) = value.as_object_mut() {
        object.remove("$schema");
    }
    Ok(value.to_string())
}

/// The final `result` event of a claude `stream-json` stream, if any. Later
/// events supersede earlier ones — a well-formed run emits exactly one — and
/// non-JSON or non-result lines are skipped.
fn last_result_event(stdout: &str) -> Option<serde_json::Value> {
    let mut last = None;
    for line in stdout.lines() {
        let Ok(event) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if event.get("type").and_then(|t| t.as_str()) == Some("result") {
            last = Some(event);
        }
    }
    last
}

/// The agent's final answer: the parsed `structured_output` object from a
/// `result` event, reserialized as JSON so the on-disk last-message file
/// matches the codex path's shape. There is deliberately no fallback to the
/// prose `result` string: every downstream consumer parses schema JSON, so
/// with `--json-schema` enforced a success without `structured_output` has no
/// usable answer, and writing prose here would only move the failure to a
/// confusing serde error at read time. Returns None in that case so the caller
/// can fail loudly naming the transcript.
fn claude_last_message(result: &serde_json::Value) -> Option<String> {
    let structured = result.get("structured_output")?;
    if structured.is_null() {
        return None;
    }
    serde_json::to_string_pretty(structured).ok()
}

/// The failure cause for a claude run, from the terminal `result` event: the
/// error subtype (e.g. `error_during_execution`) and/or the `result` string
/// that carries the error text. A `success` subtype is filtered out — a
/// process can die after emitting a success result (killed in teardown, say),
/// and a diagnostic reading "cause: success: ..." would mislead. Returns None
/// when no result event was emitted, leaving stderr as the only evidence.
fn claude_error_cause(stdout: &str) -> Option<String> {
    let result = last_result_event(stdout)?;
    let subtype = result
        .get("subtype")
        .and_then(|value| value.as_str())
        .filter(|subtype| *subtype != "success");
    let message = result.get("result").and_then(|value| value.as_str());
    match (subtype, message) {
        (Some(subtype), Some(message)) => Some(format!("{subtype}: {message}")),
        (Some(subtype), None) => Some(subtype.to_string()),
        (None, Some(message)) => Some(message.to_string()),
        (None, None) => None,
    }
}

/// Reject duplicate model-assigned identifiers before an artifact is accepted.
///
/// Finding ids are the join key for judging and matching. Keeping a malformed
/// run around until `compare` would turn a completed, paid run into an
/// artifact that can never be used.
fn ensure_unique_finding_ids(findings: &[Finding]) -> Result<()> {
    let mut seen = BTreeSet::new();
    for finding in findings {
        ensure!(
            seen.insert(&finding.id),
            "duplicate finding id '{}'",
            finding.id
        );
    }
    Ok(())
}

fn collect_run_findings(run_dir: &Path) -> Result<Vec<Finding>> {
    let metadata: RunMetadata = read_json(&run_dir.join("run.json"))?;
    let mut findings = Vec::new();
    let mut seen_ids = BTreeSet::new();
    for repeat in 1..=metadata.repeats {
        let path = run_dir.join(format!("repeat-{repeat}/findings.json"));
        ensure!(
            path.is_file(),
            "missing findings for repeat {repeat}: {}",
            path.display()
        );
        let file: FindingsFile = read_json(&path)?;
        for mut finding in file.findings {
            finding.id = format!("repeat-{repeat}:{}", finding.id);
            ensure!(
                seen_ids.insert(finding.id.clone()),
                "duplicate finding id '{}'",
                finding.id
            );
            finding.repeat = Some(repeat);
            findings.push(finding);
        }
    }
    Ok(findings)
}

fn judge_findings(
    root: &Path,
    tools: &ToolEnv,
    spec: ModelSpec,
    target: &PreparedCase,
    out_dir: &Path,
    name: &str,
    findings: &[Finding],
) -> Result<Vec<Judgment>> {
    let prompt_template = fs::read_to_string(root.join(EVAL_ROOT).join("prompts/judge.md"))?;
    let diff = git_stdout(
        tools,
        &target.checkout,
        [
            "diff",
            "--find-renames",
            &target.base_sha,
            &target.subject_sha,
        ],
    )?;
    let input = serde_json::json!({
        "base_sha": target.base_sha,
        "subject_sha": target.subject_sha,
        "findings": findings,
        "diff": diff,
    });
    let prompt = format!(
        "{prompt_template}\n\nReview input:\n\n```json\n{}\n```",
        serde_json::to_string_pretty(&input)?
    );
    let schema = root.join(EVAL_ROOT).join("schemas/judgments.schema.json");
    let output_path = out_dir.join(format!("{name}-judgments.json"));
    let transcript_path = out_dir.join(format!("{name}-judge-transcript.jsonl"));
    run_agent(
        tools,
        spec,
        &target.checkout,
        &schema,
        &output_path,
        &transcript_path,
        &prompt,
    )?;
    let judgments: JudgmentsFile = read_json(&output_path)?;
    validate_judgments(name, findings, &judgments.judgments)?;
    write_json(&output_path, &judgments)?;
    Ok(judgments.judgments)
}

fn match_findings(
    root: &Path,
    tools: &ToolEnv,
    spec: ModelSpec,
    target: &PreparedCase,
    out_dir: &Path,
    baseline: &[Finding],
    candidate: &[Finding],
) -> Result<Vec<FindingMatch>> {
    let prompt_template = fs::read_to_string(root.join(EVAL_ROOT).join("prompts/match.md"))?;
    let input = serde_json::json!({
        "baseline_findings": baseline,
        "candidate_findings": candidate,
    });
    let prompt = format!(
        "{prompt_template}\n\nMatch input:\n\n```json\n{}\n```",
        serde_json::to_string_pretty(&input)?
    );
    let schema = root.join(EVAL_ROOT).join("schemas/matches.schema.json");
    let output_path = out_dir.join("matches.json");
    let transcript_path = out_dir.join("match-transcript.jsonl");
    run_agent(
        tools,
        spec,
        &target.checkout,
        &schema,
        &output_path,
        &transcript_path,
        &prompt,
    )?;
    let matches: MatchFile = read_json(&output_path)?;
    validate_matches(baseline, candidate, &matches.matches)?;
    Ok(matches.matches)
}

fn validate_judgments(name: &str, findings: &[Finding], judgments: &[Judgment]) -> Result<()> {
    let expected: BTreeSet<_> = findings.iter().map(|finding| finding.id.as_str()).collect();
    let mut seen = BTreeSet::new();
    for judgment in judgments {
        ensure!(
            expected.contains(judgment.finding_id.as_str()),
            "{name} judge returned unknown finding id '{}'",
            judgment.finding_id
        );
        ensure!(
            seen.insert(judgment.finding_id.as_str()),
            "{name} judge returned duplicate judgment for '{}'",
            judgment.finding_id
        );
    }
    for id in expected {
        ensure!(
            seen.contains(id),
            "{name} judge omitted judgment for finding '{id}'"
        );
    }
    Ok(())
}

fn validate_matches(
    baseline: &[Finding],
    candidate: &[Finding],
    matches: &[FindingMatch],
) -> Result<()> {
    let baseline_ids: BTreeSet<_> = baseline.iter().map(|finding| finding.id.as_str()).collect();
    let candidate_ids: BTreeSet<_> = candidate
        .iter()
        .map(|finding| finding.id.as_str())
        .collect();
    let mut seen_baseline_ids = BTreeSet::new();
    let mut seen_candidate_ids = BTreeSet::new();
    for finding_match in matches {
        ensure!(
            baseline_ids.contains(finding_match.baseline_finding_id.as_str()),
            "matcher returned unknown baseline finding id '{}'",
            finding_match.baseline_finding_id
        );
        ensure!(
            candidate_ids.contains(finding_match.candidate_finding_id.as_str()),
            "matcher returned unknown candidate finding id '{}'",
            finding_match.candidate_finding_id
        );
        ensure!(
            seen_baseline_ids.insert(finding_match.baseline_finding_id.as_str()),
            "matcher returned duplicate baseline finding id '{}'",
            finding_match.baseline_finding_id
        );
        ensure!(
            seen_candidate_ids.insert(finding_match.candidate_finding_id.as_str()),
            "matcher returned duplicate candidate finding id '{}'",
            finding_match.candidate_finding_id
        );
    }
    Ok(())
}

fn build_comparison(
    baseline_meta: &RunMetadata,
    candidate_meta: &RunMetadata,
    baseline_findings: Vec<Finding>,
    candidate_findings: Vec<Finding>,
    baseline_judgments: Vec<Judgment>,
    candidate_judgments: Vec<Judgment>,
    matches: Vec<FindingMatch>,
) -> ComparisonFile {
    let good_baseline = good_ids(&baseline_judgments);
    let good_candidate = good_ids(&candidate_judgments);
    let matched_good_baseline: BTreeSet<_> = matches
        .iter()
        .filter(|m| good_candidate.contains(&m.candidate_finding_id))
        .map(|m| m.baseline_finding_id.clone())
        .collect();
    let baseline_by_id: BTreeMap<_, _> = baseline_findings
        .iter()
        .map(|f| (f.id.clone(), f))
        .collect();
    let likely_regressions = good_baseline
        .difference(&matched_good_baseline)
        .filter_map(|id| {
            baseline_by_id.get(id).map(|finding| LikelyRegression {
                baseline_finding_id: id.clone(),
                summary: finding.summary.clone(),
                rationale:
                    "Baseline finding was judged good and no matched candidate finding was judged good."
                        .to_string(),
            })
        })
        .collect();
    let nondeterminism_notes = nondeterminism_notes("baseline", &baseline_findings)
        .into_iter()
        .chain(nondeterminism_notes("candidate", &candidate_findings))
        .collect();

    ComparisonFile {
        baseline_run: baseline_meta.id.clone(),
        candidate_run: candidate_meta.id.clone(),
        case_id: baseline_meta.case_id.clone(),
        candidate_model: Some(candidate_meta.model.clone()),
        baseline_backend: baseline_meta.backend,
        candidate_backend: candidate_meta.backend,
        matches,
        likely_regressions,
        nondeterminism_notes,
    }
}

fn good_ids(judgments: &[Judgment]) -> BTreeSet<String> {
    judgments
        .iter()
        .filter(|j| j.classification == Classification::Good)
        .map(|j| j.finding_id.clone())
        .collect()
}

fn nondeterminism_notes(label: &str, findings: &[Finding]) -> Vec<String> {
    let repeats: BTreeSet<_> = findings.iter().filter_map(|f| f.repeat).collect();
    let mut by_summary: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
    for finding in findings {
        let key = format!("{} {}", finding.category, finding.summary).to_lowercase();
        if let Some(repeat) = finding.repeat {
            by_summary.entry(key).or_default().insert(repeat);
        }
    }
    by_summary
        .into_iter()
        .filter(|(_, seen)| seen.len() < repeats.len())
        .map(|(summary, seen)| {
            format!(
                "{label} finding appeared in {}/{} repeats: {summary}",
                seen.len(),
                repeats.len()
            )
        })
        .collect()
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))?;
    Ok(())
}

/// Resolve a case ref to a commit SHA, preferring the origin branch of that
/// name so a stale local checkout never wins over the pushed state.
///
/// Falls back to `git fetch origin <reference>` when the ref resolves neither
/// as an origin branch nor locally. Mined cases pin exact SHAs whose commits
/// are often reachable only from GitHub's `refs/pull/N/head`, which clones do
/// not fetch; GitHub serves fetch-by-SHA for such commits, so the fetch makes
/// the pinned commit resolvable. Force-pushed-away commits are eventually
/// GC'd upstream and will fail here — cases must pin commits that remain
/// ancestors of a retained ref.
fn resolve_case_ref(tools: &ToolEnv, repo: &Path, reference: &str) -> Result<String> {
    let remote_ref = format!("refs/remotes/origin/{reference}^{{commit}}");
    if let Some(sha) = git_stdout_optional(
        tools,
        repo,
        &["rev-parse", "--verify", "--quiet", &remote_ref],
    )? {
        return Ok(sha);
    }
    let commit_ref = format!("{reference}^{{commit}}");
    if let Some(sha) = git_stdout_optional(
        tools,
        repo,
        &["rev-parse", "--verify", "--quiet", &commit_ref],
    )? {
        return Ok(sha);
    }
    run_checked(
        Command::new(&tools.git)
            .arg("-C")
            .arg(repo)
            .args(["fetch", "origin"])
            .arg(reference),
    )
    .with_context(|| format!("failed to fetch unresolvable ref '{reference}' from origin"))?;
    git_stdout(tools, repo, ["rev-parse", &commit_ref])
}

fn git_stdout<const N: usize>(tools: &ToolEnv, repo: &Path, args: [&str; N]) -> Result<String> {
    let output = Command::new(&tools.git)
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .context("failed to run git")?;
    ensure!(
        output.status.success(),
        "git failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn git_stdout_optional(tools: &ToolEnv, repo: &Path, args: &[&str]) -> Result<Option<String>> {
    let output = Command::new(&tools.git)
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .context("failed to run git")?;
    if output.status.success() {
        Ok(Some(String::from_utf8(output.stdout)?.trim().to_string()))
    } else {
        Ok(None)
    }
}

fn run_checked(command: &mut Command) -> Result<()> {
    let output = command.output().context("failed to run command")?;
    ensure!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn absolutize(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn slug(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

fn unique_id(prefix: &str) -> Result<String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(format!("{prefix}-{now}"))
}

fn now_rfc3339() -> Result<String> {
    Ok(OffsetDateTime::now_utc().format(&Rfc3339)?)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn slug_normalizes_refs_for_paths() {
        assert_eq!(slug("scode/treeward"), "scode-treeward");
        assert_eq!(slug("pr/narrow correctness"), "pr-narrow-correctness");
    }

    #[test]
    fn comparison_flags_good_baseline_without_good_candidate_match() {
        let baseline_meta = run_meta("baseline", "model-a");
        let candidate_meta = run_meta("candidate", "model-a");
        let baseline_findings = vec![finding("B1", "real bug", 1)];
        let candidate_findings = vec![finding("C1", "different bug", 1)];
        let baseline_judgments = vec![judgment("B1", Classification::Good)];
        let candidate_judgments = vec![judgment("C1", Classification::Good)];

        let comparison = build_comparison(
            &baseline_meta,
            &candidate_meta,
            baseline_findings,
            candidate_findings,
            baseline_judgments,
            candidate_judgments,
            Vec::new(),
        );

        assert_eq!(comparison.likely_regressions.len(), 1);
        assert_eq!(comparison.likely_regressions[0].baseline_finding_id, "B1");
    }

    #[test]
    fn comparison_does_not_flag_matched_good_candidate() {
        let baseline_meta = run_meta("baseline", "model-a");
        let candidate_meta = run_meta("candidate", "model-a");
        let comparison = build_comparison(
            &baseline_meta,
            &candidate_meta,
            vec![finding("B1", "real bug", 1)],
            vec![finding("C1", "real bug", 1)],
            vec![judgment("B1", Classification::Good)],
            vec![judgment("C1", Classification::Good)],
            vec![FindingMatch {
                baseline_finding_id: "B1".to_string(),
                candidate_finding_id: "C1".to_string(),
                rationale: "same issue".to_string(),
            }],
        );

        assert!(comparison.likely_regressions.is_empty());
    }

    #[test]
    fn comparison_ignores_non_good_baseline_findings() {
        let baseline_meta = run_meta("baseline", "model-a");
        let candidate_meta = run_meta("candidate", "model-a");
        let comparison = build_comparison(
            &baseline_meta,
            &candidate_meta,
            vec![
                finding("B1", "wrong baseline finding", 1),
                finding("B2", "unclear baseline finding", 1),
            ],
            Vec::new(),
            vec![
                judgment("B1", Classification::Incorrect),
                judgment("B2", Classification::Indeterminate),
            ],
            Vec::new(),
            Vec::new(),
        );

        assert!(comparison.likely_regressions.is_empty());
    }

    #[test]
    fn comparison_requires_matched_candidate_to_be_good() {
        let baseline_meta = run_meta("baseline", "model-a");
        let candidate_meta = run_meta("candidate", "model-a");
        let comparison = build_comparison(
            &baseline_meta,
            &candidate_meta,
            vec![finding("B1", "real bug", 1)],
            vec![finding("C1", "same reported bug", 1)],
            vec![judgment("B1", Classification::Good)],
            vec![judgment("C1", Classification::Incorrect)],
            vec![FindingMatch {
                baseline_finding_id: "B1".to_string(),
                candidate_finding_id: "C1".to_string(),
                rationale: "same issue".to_string(),
            }],
        );

        assert_eq!(comparison.likely_regressions.len(), 1);
        assert_eq!(comparison.likely_regressions[0].baseline_finding_id, "B1");
    }

    #[test]
    fn run_command_writes_artifacts_with_fake_git_and_codex() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        write_eval_fixture(root)?;
        let _fake_bin_guard = fake_bin_lock();
        write_fake_bin(root, "git", fake_git_script())?;
        write_fake_bin(root, "codex", fake_codex_script())?;
        let tools = fake_tools(root);

        RunCommand {
            skill: DEFAULT_SKILL.to_string(),
            case_id: "case-a".to_string(),
            model: "fake-model".to_string(),
            repeats: 2,
            label: "smoke".to_string(),
            skill_ref: None,
            skill_path: Some(root.join(DEFAULT_SKILL_PATH)),
            reviewer: None,
            backend: Backend::Codex,
            effort: None,
        }
        .run_with_tools(root, &tools)?;

        let run_root = root.join(RUN_ROOT);
        let run_dir = fs::read_dir(&run_root)?
            .next()
            .expect("expected run dir")?
            .path();
        assert!(run_dir.join("run.json").is_file());
        assert!(run_dir.join("scope.diff").is_file());
        // Unrestricted runs preserve the coordinator transcript and raw
        // structured result. Reviewer details live in execution.json instead
        // of pretending the harness directly ran those agents.
        for repeat in ["repeat-1", "repeat-2"] {
            assert!(run_dir.join(repeat).join("swarm-result.json").is_file());
            assert!(run_dir.join(repeat).join("transcript.jsonl").is_file());
        }
        let execution: ExecutionRecord = read_json(&run_dir.join("repeat-1/execution.json"))?;
        assert_eq!(execution.expected, 2);
        assert_eq!(execution.reviewers.len(), 2);
        assert_eq!(execution.skipped_reviewers.len(), 1);
        assert_eq!(execution.skipped_reviewers[0].reviewer, "spec-compliance");
        assert_eq!(execution.coordinator.as_ref().unwrap().spawned_agents, 2);
        assert_eq!(execution.coordinator.as_ref().unwrap().followups, 1);
        let findings: FindingsFile = read_json(&run_dir.join("repeat-1/findings.json"))?;
        assert_eq!(findings.findings.len(), 1);
        assert_eq!(
            findings.findings[0].reviewers.as_deref(),
            Some(&["docs-comments".to_string()][..])
        );
        let metadata: RunMetadata = read_json(&run_dir.join("run.json"))?;
        assert_eq!(metadata.base_sha, "base-sha");
        assert_eq!(metadata.subject_sha, "subject-remote-sha");
        assert_eq!(metadata.execution_mode, ExecutionMode::Swarm);

        BaselineCommand {
            run: run_dir.strip_prefix(root)?.to_path_buf(),
        }
        .run(root)?;
        assert!(run_dir.join("baseline.json").is_file());

        Ok(())
    }

    /// A schema-valid answer from a coordinator that never delegated is not a
    /// swarm. The transcript audit is the guard that the original
    /// coordinator-owned harness lacked; keep the failure evidence on disk so
    /// the next incident is diagnosable without rerunning it.
    #[test]
    fn swarm_run_rejects_solo_coordinator() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        write_eval_fixture(root)?;
        let _fake_bin_guard = fake_bin_lock();
        write_fake_bin(root, "git", fake_git_script())?;
        write_fake_bin(
            root,
            "codex",
            r#"#!/usr/bin/env bash
out=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "-o" ]; then out="$2"; shift 2; else shift; fi
done
cat >/dev/null
mkdir -p "$(dirname "$out")"
case "$(basename "$out")" in
  preflight-*)
    printf '{"findings":[{"id":"P1","category":"correctness","summary":"planted","location":"src/even.rs:3","rationale":"planted"}]}' >"$out"
    ;;
  *)
    printf '%s' '{"findings":[],"reviewer_execution":[{"reviewer":"docs-comments","status":"completed","passes":1,"rationale":""},{"reviewer":"test-quality","status":"completed","passes":1,"rationale":""}]}' >"$out"
    ;;
esac
echo '{"type":"turn.completed","usage":{"output_tokens":42}}'
"#,
        )?;
        let tools = fake_tools(root);

        let error = RunCommand {
            skill: DEFAULT_SKILL.to_string(),
            case_id: "case-a".to_string(),
            model: "fake-model".to_string(),
            repeats: 1,
            label: "solo".to_string(),
            skill_ref: None,
            skill_path: Some(root.join(DEFAULT_SKILL_PATH)),
            reviewer: None,
            backend: Backend::Codex,
            effort: None,
        }
        .run_with_tools(root, &tools)
        .unwrap_err();

        assert!(error.to_string().contains("spawned 0 reviewer agents"));
        let run_dir = fs::read_dir(root.join(RUN_ROOT))?
            .next()
            .expect("expected run dir")?
            .path();
        let execution: ExecutionRecord = read_json(&run_dir.join("repeat-1/execution.json"))?;
        assert_eq!(execution.coordinator.unwrap().spawned_agents, 0);
        Ok(())
    }

    /// The --reviewer restriction must actually shrink the panel the harness
    /// spawns — not just be recorded — and must land in run.json so compare
    /// can refuse to mix restricted and unrestricted runs. A restriction that
    /// silently failed to filter would spend the full panel and label the
    /// result single-reviewer.
    #[test]
    fn reviewer_restriction_reaches_prompt_and_metadata() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        write_eval_fixture(root)?;
        let _fake_bin_guard = fake_bin_lock();
        write_fake_bin(root, "git", fake_git_script())?;
        write_fake_bin(root, "codex", fake_codex_script())?;
        let tools = fake_tools(root);

        RunCommand {
            skill: DEFAULT_SKILL.to_string(),
            case_id: "case-a".to_string(),
            model: "fake-model".to_string(),
            repeats: 1,
            label: "restricted".to_string(),
            skill_ref: None,
            skill_path: Some(root.join(DEFAULT_SKILL_PATH)),
            reviewer: Some("test-quality".to_string()),
            backend: Backend::Codex,
            effort: None,
        }
        .run_with_tools(root, &tools)?;

        let run_dir = fs::read_dir(root.join(RUN_ROOT))?
            .next()
            .expect("expected run dir")?
            .path();
        let metadata: RunMetadata = read_json(&run_dir.join("run.json"))?;
        assert_eq!(metadata.reviewer.as_deref(), Some("test-quality"));
        let execution: ExecutionRecord = read_json(&run_dir.join("repeat-1/execution.json"))?;
        assert_eq!(execution.expected, 1);
        assert_eq!(execution.reviewers[0].reviewer, "test-quality");
        assert!(
            !run_dir
                .join("repeat-1/reviewers/docs-comments.findings.json")
                .exists()
        );
        let prompt = fs::read_to_string(
            run_dir.join("repeat-1/reviewers/test-quality.findings.json.prompt"),
        )?;
        assert!(prompt.contains("reviewers/test-quality.md"));
        Ok(())
    }

    /// Restricting to a shared base charter must fail before any tokens are
    /// spent: base charters are reading material for lens reviewers, and
    /// spawning one as a reviewer would silently measure a phantom panel
    /// member.
    #[test]
    fn run_command_rejects_base_charter_restriction() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        write_eval_fixture(root)?;
        let _fake_bin_guard = fake_bin_lock();
        write_fake_bin(root, "git", fake_git_script())?;
        let tools = fake_tools(root);

        let error = RunCommand {
            skill: DEFAULT_SKILL.to_string(),
            case_id: "case-a".to_string(),
            model: "fake-model".to_string(),
            repeats: 1,
            label: "restricted".to_string(),
            skill_ref: None,
            skill_path: Some(root.join(DEFAULT_SKILL_PATH)),
            reviewer: Some("correctness".to_string()),
            backend: Backend::Codex,
            effort: None,
        }
        .run_with_tools(root, &tools)
        .unwrap_err();

        assert!(error.to_string().contains("not spawnable"));
        Ok(())
    }

    /// Panel discovery is driven by markers in the charter files themselves,
    /// so it must hold across skill versions: base charters are never
    /// spawned, and SPEC.md-gated charters run only when the target checkout
    /// actually has a SPEC.md. A discovery bug here silently changes what a
    /// run measures.
    #[test]
    fn discover_panel_applies_charter_markers() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        write_eval_fixture(root)?;
        let skill_path = root.join(DEFAULT_SKILL_PATH);
        let target_root = root.join("target-checkout");
        fs::create_dir_all(&target_root)?;

        let panel = discover_panel(&skill_path, &target_root)?;
        assert_eq!(panel, vec!["docs-comments", "test-quality"]);
        let known = known_reviewers(&skill_path)?;
        assert_eq!(
            known,
            vec!["docs-comments", "spec-compliance", "test-quality"]
        );
        let aliases = reviewer_aliases(&skill_path, &known)?;
        assert_eq!(
            canonical_reviewer(&aliases, "docs-comments-reviewer")?,
            "docs-comments"
        );
        assert_eq!(
            canonical_reviewer(&aliases, "test-quality")?,
            "test-quality"
        );
        assert_eq!(
            canonical_reviewer(&aliases, "spec-compliance-reviewer")?,
            "spec-compliance"
        );

        fs::write(target_root.join("SPEC.md"), "# spec")?;
        let panel = discover_panel(&skill_path, &target_root)?;
        assert_eq!(
            panel,
            vec!["docs-comments", "spec-compliance", "test-quality"]
        );
        Ok(())
    }

    /// Reviewer aliases must not depend on charter enumeration order. A title
    /// that collides with another charter's basename is ambiguous even if the
    /// conflicting basename happens to be visited later.
    #[test]
    fn reviewer_aliases_reject_title_and_machine_key_collisions() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        write_eval_fixture(root)?;
        let skill_path = root.join(DEFAULT_SKILL_PATH);
        fs::write(
            skill_path.join("reviewers/docs-comments.md"),
            "# test-quality",
        )?;

        let known = known_reviewers(&skill_path)?;
        let error = reviewer_aliases(&skill_path, &known).unwrap_err();

        assert!(error.to_string().contains("reviewer alias 'test-quality'"));
        Ok(())
    }

    /// A reviewer agent that fails must abort the whole run, naming the
    /// reviewer. Carrying on with a partial panel would quietly produce the
    /// missing-reviewer data this fan-out design exists to make impossible.
    #[test]
    fn reviewer_failure_aborts_run() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        write_eval_fixture(root)?;
        let _fake_bin_guard = fake_bin_lock();
        write_fake_bin(root, "git", fake_git_script())?;
        // Preflight calls must succeed so the failure under test is the
        // reviewer fan-out, not the preflight guard that runs before it.
        write_fake_bin(
            root,
            "codex",
            r#"#!/usr/bin/env bash
out=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "-o" ]; then out="$2"; shift 2; else shift; fi
done
cat >/dev/null
case "$(basename "$out")" in
  preflight-*)
    mkdir -p "$(dirname "$out")"
    printf '{"findings":[{"id":"P1","category":"correctness","summary":"planted","location":"src/even.rs:3","rationale":"planted"}]}' >"$out"
    ;;
  *)
    echo 'fake reviewer crash' >&2
    exit 1
    ;;
esac
"#,
        )?;
        let tools = fake_tools(root);

        let error = RunCommand {
            skill: DEFAULT_SKILL.to_string(),
            case_id: "case-a".to_string(),
            model: "fake-model".to_string(),
            repeats: 1,
            label: "crash".to_string(),
            skill_ref: None,
            skill_path: Some(root.join(DEFAULT_SKILL_PATH)),
            reviewer: Some("test-quality".to_string()),
            backend: Backend::Codex,
            effort: None,
        }
        .run_with_tools(root, &tools)
        .unwrap_err();

        let message = format!("{error:#}");
        assert!(message.contains("reviewer '"));
        assert!(message.contains("failed"));
        Ok(())
    }

    /// Every run must end with a verification digest whose numbers come from
    /// the on-disk transcripts, because the digest is what the launching
    /// agent inspects instead of reading every raw transcript. A digest that
    /// silently miscounted tokens or commands would defeat the inspection
    /// step it exists to enable.
    #[test]
    fn verification_digest_reflects_transcript_evidence() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        write_eval_fixture(root)?;
        let _fake_bin_guard = fake_bin_lock();
        write_fake_bin(root, "git", fake_git_script())?;
        write_fake_bin(root, "codex", fake_codex_script())?;
        let tools = fake_tools(root);

        RunCommand {
            skill: DEFAULT_SKILL.to_string(),
            case_id: "case-a".to_string(),
            model: "fake-model".to_string(),
            repeats: 2,
            label: "smoke".to_string(),
            skill_ref: None,
            skill_path: Some(root.join(DEFAULT_SKILL_PATH)),
            reviewer: None,
            backend: Backend::Codex,
            effort: None,
        }
        .run_with_tools(root, &tools)?;

        let run_dir = fs::read_dir(root.join(RUN_ROOT))?
            .next()
            .expect("expected run dir")?
            .path();
        let verification: RunVerification = read_json(&run_dir.join("verification.json"))?;
        assert_eq!(verification.status, "clean");
        assert_eq!(verification.anomaly_count, 0);
        assert_eq!(verification.repeats.len(), 2);
        for repeat in &verification.repeats {
            assert_eq!(repeat.expected_reviewers, 2);
            assert_eq!(repeat.completed_reviewers, 2);
            let coordinator = repeat.coordinator.as_ref().unwrap();
            assert_eq!(coordinator.output_tokens, Some(42));
            assert_eq!(coordinator.commands, 1);
            assert_eq!(coordinator.spawned_agents, 2);
            assert_eq!(coordinator.followups, 1);
            assert!(coordinator.anomalies.is_empty());
            for reviewer in &repeat.reviewers {
                assert_eq!(reviewer.output_tokens, None);
                assert_eq!(reviewer.commands, 0);
                assert!(reviewer.anomalies.is_empty());
            }
        }
        Ok(())
    }

    /// The anomaly signals must fire on the transcript shapes that indicate
    /// no real agent work: a missing/unreadable transcript, a stream that
    /// never completed a turn, and a turn that produced zero output. These
    /// are the post-run analogues of the solo-swarm incident — execution that
    /// looks finished but did nothing.
    #[test]
    fn transcript_digest_flags_empty_or_missing_work() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let dir = temp.path();

        let (tokens, commands, anomalies) =
            digest_transcript(Backend::Codex, &dir.join("missing.jsonl"));
        assert_eq!((tokens, commands), (None, 0));
        assert_eq!(anomalies.len(), 1);
        assert!(anomalies[0].contains("missing"));

        let no_turn = dir.join("no-turn.jsonl");
        fs::write(&no_turn, "{\"event\":\"done\"}\nnot-json\n")?;
        let (tokens, _, anomalies) = digest_transcript(Backend::Codex, &no_turn);
        assert_eq!(tokens, None);
        assert!(anomalies[0].contains("no turn.completed"));

        let zero = dir.join("zero.jsonl");
        fs::write(
            &zero,
            "{\"type\":\"turn.completed\",\"usage\":{\"output_tokens\":0}}\n",
        )?;
        let (tokens, _, anomalies) = digest_transcript(Backend::Codex, &zero);
        assert_eq!(tokens, Some(0));
        assert!(anomalies[0].contains("zero output tokens"));

        let healthy = dir.join("healthy.jsonl");
        fs::write(
            &healthy,
            "{\"type\":\"item.completed\",\"item\":{\"type\":\"command_execution\"}}\n\
             {\"type\":\"turn.completed\",\"usage\":{\"output_tokens\":7}}\n",
        )?;
        let (tokens, commands, anomalies) = digest_transcript(Backend::Codex, &healthy);
        assert_eq!((tokens, commands), (Some(7), 1));
        assert!(anomalies.is_empty());
        Ok(())
    }

    /// Failed collaboration attempts are not evidence that a reviewer ran or
    /// that a continuation pass happened. Codex emits those attempts as
    /// `item.completed` records too, so the nested status must gate both
    /// counters.
    #[test]
    fn collaboration_digest_ignores_failed_codex_calls() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let transcript = temp.path().join("collaboration.jsonl");
        fs::write(
            &transcript,
            concat!(
                "{\"type\":\"item.completed\",\"item\":{\"type\":\"collab_tool_call\",",
                "\"tool\":\"spawn_agent\",\"receiver_thread_ids\":[\"ok\"],",
                "\"status\":\"completed\"}}\n",
                "{\"type\":\"item.completed\",\"item\":{\"type\":\"collab_tool_call\",",
                "\"tool\":\"spawn_agent\",\"receiver_thread_ids\":[\"failed\"],",
                "\"status\":\"failed\"}}\n",
                "{\"type\":\"item.completed\",\"item\":{\"type\":\"collab_tool_call\",",
                "\"tool\":\"send_input\",\"receiver_thread_ids\":[\"ok\"],",
                "\"status\":\"failed\"}}\n",
            ),
        )?;

        assert_eq!(
            digest_collaboration(Backend::Codex, &transcript)?,
            CollaborationDigest {
                spawned_agents: 1,
                followups: 0,
            }
        );
        Ok(())
    }

    /// Claude records both the attempted tool call and its later result. Only
    /// a non-error result proves that a reviewer or continuation actually ran;
    /// counting attempts would reject allowed retries and let solo fallback
    /// masquerade as a swarm when every spawn failed.
    #[test]
    fn collaboration_digest_ignores_failed_claude_calls() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let transcript = temp.path().join("collaboration.jsonl");
        fs::write(
            &transcript,
            concat!(
                "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"tool_use\",",
                "\"id\":\"spawn-ok\",\"name\":\"Agent\"}]}}\n",
                "{\"type\":\"user\",\"message\":{\"content\":[{\"type\":\"tool_result\",",
                "\"tool_use_id\":\"spawn-ok\",\"is_error\":false}]}}\n",
                "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"tool_use\",",
                "\"id\":\"spawn-failed\",\"name\":\"Agent\"}]}}\n",
                "{\"type\":\"user\",\"message\":{\"content\":[{\"type\":\"tool_result\",",
                "\"tool_use_id\":\"spawn-failed\",\"is_error\":true}]}}\n",
                "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"tool_use\",",
                "\"id\":\"followup-ok\",\"name\":\"SendMessage\"}]}}\n",
                "{\"type\":\"user\",\"message\":{\"content\":[{\"type\":\"tool_result\",",
                "\"tool_use_id\":\"followup-ok\"}]}}\n",
                "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"tool_use\",",
                "\"id\":\"followup-without-result\",\"name\":\"SendMessage\"}]}}\n",
            ),
        )?;

        assert_eq!(
            digest_collaboration(Backend::Claude, &transcript)?,
            CollaborationDigest {
                spawned_agents: 1,
                followups: 1,
            }
        );
        Ok(())
    }

    /// Every run must leave preflight evidence on disk: the record is what
    /// lets the invoking agent verify that concurrent agent execution with
    /// structured output actually worked before trusting the run's findings.
    #[test]
    fn preflight_records_evidence_on_success() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        write_eval_fixture(root)?;
        let _fake_bin_guard = fake_bin_lock();
        write_fake_bin(root, "git", fake_git_script())?;
        write_fake_bin(root, "codex", fake_codex_script())?;
        let tools = fake_tools(root);

        RunCommand {
            skill: DEFAULT_SKILL.to_string(),
            case_id: "case-a".to_string(),
            model: "fake-model".to_string(),
            repeats: 1,
            label: "smoke".to_string(),
            skill_ref: None,
            skill_path: Some(root.join(DEFAULT_SKILL_PATH)),
            reviewer: None,
            backend: Backend::Codex,
            effort: None,
        }
        .run_with_tools(root, &tools)?;

        let run_dir = fs::read_dir(root.join(RUN_ROOT))?
            .next()
            .expect("expected run dir")?
            .path();
        let record: PreflightRecord = read_json(&run_dir.join("preflight/preflight.json"))?;
        assert_eq!(record.status, "passed");
        assert_eq!(record.agents.len(), PREFLIGHT_AGENTS);
        for agent in &record.agents {
            assert!(agent.planted_issue_found);
            assert!(
                run_dir
                    .join("preflight")
                    .join(format!("{}.transcript.jsonl", agent.agent))
                    .is_file()
            );
        }
        Ok(())
    }

    /// A failed codex exec must surface the cause from the --json stream:
    /// codex writes errors to stdout events, so without this the CLI error is
    /// the useless "codex exec failed: " with empty stderr that made the
    /// schema-rejection and bogus-model failures needlessly hard to diagnose.
    #[test]
    fn codex_failure_surfaces_last_error_event_and_transcript() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        write_eval_fixture(root)?;
        let _fake_bin_guard = fake_bin_lock();
        write_fake_bin(root, "git", fake_git_script())?;
        // Emits both error shapes on stdout, then fails with empty stderr —
        // the observed real-world failure mode.
        write_fake_bin(
            root,
            "codex",
            r#"#!/usr/bin/env bash
cat >/dev/null
echo '{"type":"item.completed","item":{"id":"item_0","type":"error","message":"model metadata missing"}}'
echo '{"type":"error","message":"api rejected request: status 400"}'
exit 1
"#,
        )?;
        let tools = fake_tools(root);

        let error = RunCommand {
            skill: DEFAULT_SKILL.to_string(),
            case_id: "case-a".to_string(),
            model: "fake-model".to_string(),
            repeats: 1,
            label: "crash".to_string(),
            skill_ref: None,
            skill_path: Some(root.join(DEFAULT_SKILL_PATH)),
            reviewer: None,
            backend: Backend::Codex,
            effort: None,
        }
        .run_with_tools(root, &tools)
        .unwrap_err();

        let message = format!("{error:#}");
        assert!(message.contains("api rejected request: status 400"));
        assert!(message.contains("transcript:"));
        Ok(())
    }

    /// The two error-event shapes and the last-one-wins rule, pinned at the
    /// parser level: a stream whose final signal is an item-shaped error must
    /// not be shadowed by an earlier top-level one, and vice versa.
    #[test]
    fn last_error_event_prefers_final_error_of_either_shape() {
        let stream = concat!(
            "{\"type\":\"error\",\"message\":\"first\"}\n",
            "not-json\n",
            "{\"type\":\"item.completed\",\"item\":{\"type\":\"error\",\"message\":\"second\"}}\n",
        );
        assert_eq!(last_error_event(stream).as_deref(), Some("second"));
        assert_eq!(last_error_event("{\"type\":\"turn.started\"}\n"), None);
    }

    /// A hard preflight agent failure (codex crash, bad model) must still
    /// leave preflight.json on disk. The run output points readers at that
    /// record, and an outright failure is exactly when someone goes looking —
    /// a record written only on the softer miss path would be missing when it
    /// matters most. (Found by manual testing with a bogus model id.)
    #[test]
    fn preflight_agent_failure_still_writes_record() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        write_eval_fixture(root)?;
        let _fake_bin_guard = fake_bin_lock();
        write_fake_bin(root, "git", fake_git_script())?;
        write_fake_bin(
            root,
            "codex",
            "#!/usr/bin/env bash\ncat >/dev/null\necho 'fake codex crash' >&2\nexit 1\n",
        )?;
        let tools = fake_tools(root);

        let error = RunCommand {
            skill: DEFAULT_SKILL.to_string(),
            case_id: "case-a".to_string(),
            model: "fake-model".to_string(),
            repeats: 1,
            label: "crash".to_string(),
            skill_ref: None,
            skill_path: Some(root.join(DEFAULT_SKILL_PATH)),
            reviewer: None,
            backend: Backend::Codex,
            effort: None,
        }
        .run_with_tools(root, &tools)
        .unwrap_err();

        let message = format!("{error:#}");
        assert!(
            message.contains("preflight agent"),
            "unexpected error: {message}"
        );
        let run_dir = fs::read_dir(root.join(RUN_ROOT))?
            .next()
            .expect("expected run dir")?
            .path();
        let record: PreflightRecord = read_json(&run_dir.join("preflight/preflight.json"))?;
        assert_eq!(record.status, "failed");
        assert_eq!(record.agents.len(), PREFLIGHT_AGENTS);
        for agent in &record.agents {
            assert!(!agent.planted_issue_found);
            assert!(agent.error.as_deref().unwrap_or("").contains("failed"));
        }
        assert!(!run_dir.join("repeat-1").exists());
        Ok(())
    }

    /// A preflight agent that runs but misses the planted issue means the
    /// execution path cannot be trusted (wrong model behavior, degraded
    /// output, unread scope). The run must abort before any real-eval spend,
    /// and the failure record must still be written for diagnosis.
    #[test]
    fn preflight_miss_aborts_run_before_reviewers() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        write_eval_fixture(root)?;
        let _fake_bin_guard = fake_bin_lock();
        write_fake_bin(root, "git", fake_git_script())?;
        // Preflight agents return a finding that never references the
        // planted file — execution "worked" but the review evidently did not.
        write_fake_bin(
            root,
            "codex",
            r#"#!/usr/bin/env bash
out=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "-o" ]; then out="$2"; shift 2; else shift; fi
done
cat >/dev/null
mkdir -p "$(dirname "$out")"
printf '{"findings":[{"id":"X1","category":"correctness","summary":"generic","location":"src/other.rs:1","rationale":"generic"}]}' >"$out"
"#,
        )?;
        let tools = fake_tools(root);

        let error = RunCommand {
            skill: DEFAULT_SKILL.to_string(),
            case_id: "case-a".to_string(),
            model: "fake-model".to_string(),
            repeats: 1,
            label: "smoke".to_string(),
            skill_ref: None,
            skill_path: Some(root.join(DEFAULT_SKILL_PATH)),
            reviewer: None,
            backend: Backend::Codex,
            effort: None,
        }
        .run_with_tools(root, &tools)
        .unwrap_err();

        assert!(error.to_string().contains("preflight failed"));
        let run_dir = fs::read_dir(root.join(RUN_ROOT))?
            .next()
            .expect("expected run dir")?
            .path();
        let record: PreflightRecord = read_json(&run_dir.join("preflight/preflight.json"))?;
        assert_eq!(record.status, "failed");
        // No reviewer spend happened: the repeats loop never ran.
        assert!(!run_dir.join("repeat-1").exists());
        Ok(())
    }

    /// A typo'd reviewer name must fail before any tokens are spent, not
    /// silently run the full panel with a no-op restriction note.
    #[test]
    fn run_command_rejects_unknown_reviewer() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        write_eval_fixture(root)?;
        let _fake_bin_guard = fake_bin_lock();
        write_fake_bin(root, "git", fake_git_script())?;
        let tools = fake_tools(root);

        let error = RunCommand {
            skill: DEFAULT_SKILL.to_string(),
            case_id: "case-a".to_string(),
            model: "fake-model".to_string(),
            repeats: 1,
            label: "restricted".to_string(),
            skill_ref: None,
            skill_path: Some(root.join(DEFAULT_SKILL_PATH)),
            reviewer: Some("no-such-reviewer".to_string()),
            backend: Backend::Codex,
            effort: None,
        }
        .run_with_tools(root, &tools)
        .unwrap_err();

        assert!(error.to_string().contains("has no charter"));
        Ok(())
    }

    /// --effort must reach both artifacts that matter: the codex command line
    /// (the `-c model_reasoning_effort=...` override is the only thing that
    /// actually changes agent behavior, since --ignore-user-config strips the
    /// config file where effort would normally live) and run.json (so compare
    /// can refuse to mix runs measured at different efforts).
    #[test]
    fn effort_reaches_codex_config_and_metadata() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        write_eval_fixture(root)?;
        let _fake_bin_guard = fake_bin_lock();
        write_fake_bin(root, "git", fake_git_script())?;
        write_fake_bin(root, "codex", fake_codex_script())?;
        let tools = fake_tools(root);

        RunCommand {
            skill: DEFAULT_SKILL.to_string(),
            case_id: "case-a".to_string(),
            model: "fake-model".to_string(),
            repeats: 1,
            label: "high-effort".to_string(),
            skill_ref: None,
            skill_path: Some(root.join(DEFAULT_SKILL_PATH)),
            reviewer: None,
            backend: Backend::Codex,
            effort: Some("high".to_string()),
        }
        .run_with_tools(root, &tools)?;

        let run_dir = fs::read_dir(root.join(RUN_ROOT))?
            .next()
            .expect("expected run dir")?
            .path();
        let metadata: RunMetadata = read_json(&run_dir.join("run.json"))?;
        assert_eq!(metadata.effort.as_deref(), Some("high"));
        let config = fs::read_to_string(run_dir.join("repeat-1/swarm-result.json.config"))?;
        assert!(config.contains("model_reasoning_effort=high"));
        assert!(config.contains("agents.max_concurrent_threads_per_session=2"));
        Ok(())
    }

    /// A typo'd effort must fail before any tokens are spent, mirroring the
    /// unknown-reviewer guard: the value is passed straight to codex, which
    /// would otherwise fail (or silently coerce) mid-run after the checkout
    /// work already happened.
    #[test]
    fn run_command_rejects_unknown_effort() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let error = RunCommand {
            skill: DEFAULT_SKILL.to_string(),
            case_id: "case-a".to_string(),
            model: "fake-model".to_string(),
            repeats: 1,
            label: "smoke".to_string(),
            skill_ref: None,
            skill_path: None,
            reviewer: None,
            backend: Backend::Codex,
            effort: Some("extreme".to_string()),
        }
        .run_with_tools(root, &fake_tools(root))
        .unwrap_err();

        assert!(error.to_string().contains("unknown effort"));
        Ok(())
    }

    /// TEMPORARY (with the sandbox bypass): mined cases target third-party
    /// repos and must not run while agents execute unsandboxed. The guard has
    /// to fire before any checkout or codex spend. Delete this test when
    /// sandboxing is re-enabled and the restriction is lifted.
    #[test]
    fn run_command_rejects_mined_cases_while_unsandboxed() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        write_eval_fixture(root)?;
        let tools = fake_tools(root);

        let error = RunCommand {
            skill: DEFAULT_SKILL.to_string(),
            case_id: "case-mined".to_string(),
            model: "fake-model".to_string(),
            repeats: 1,
            label: "smoke".to_string(),
            skill_ref: None,
            skill_path: Some(root.join(DEFAULT_SKILL_PATH)),
            reviewer: None,
            backend: Backend::Codex,
            effort: None,
        }
        .run_with_tools(root, &tools)
        .unwrap_err();

        assert!(error.to_string().contains("not hand-curated"));
        Ok(())
    }

    #[test]
    fn explicit_base_ref_is_resolved() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let _fake_bin_guard = fake_bin_lock();
        write_fake_bin(root, "git", fake_git_script())?;
        let tools = fake_tools(root);
        let case = EvalCase {
            id: "case-a".to_string(),
            repo: "owner/repo".to_string(),
            subject_ref: "subject".to_string(),
            base_ref: Some("explicit-base".to_string()),
            curation: None,
        };

        let prepared = prepare_case_checkout(root, &case, &tools)?;

        assert_eq!(prepared.subject_sha, "subject-remote-sha");
        assert_eq!(prepared.base_ref, "explicit-base");
        assert_eq!(prepared.base_sha, "explicit-base-remote-sha");
        Ok(())
    }

    #[test]
    fn cached_branch_refs_resolve_from_origin() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let checkout = root.join(WORKTREE_ROOT).join("repos/owner-repo");
        fs::create_dir_all(checkout.join(".git"))?;
        let _fake_bin_guard = fake_bin_lock();
        write_fake_bin(root, "git", fake_git_script())?;
        let tools = fake_tools(root);
        let case = EvalCase {
            id: "case-a".to_string(),
            repo: "owner/repo".to_string(),
            subject_ref: "subject".to_string(),
            base_ref: None,
            curation: None,
        };

        let prepared = prepare_case_checkout(root, &case, &tools)?;

        assert_eq!(prepared.subject_sha, "subject-remote-sha");
        assert_eq!(
            fs::read_to_string(checkout.join(".fake-head-ref"))?,
            "subject-remote-sha\n"
        );
        Ok(())
    }

    #[test]
    fn unresolvable_refs_are_fetched_from_origin() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let _fake_bin_guard = fake_bin_lock();
        write_fake_bin(root, "git", fake_git_script())?;
        let tools = fake_tools(root);
        let case = EvalCase {
            id: "case-a".to_string(),
            repo: "owner/repo".to_string(),
            subject_ref: "pinned".to_string(),
            base_ref: Some("base".to_string()),
            curation: Some(Curation::Mined),
        };

        let prepared = prepare_case_checkout(root, &case, &tools)?;

        assert_eq!(prepared.subject_sha, "pinned-fetched-sha");
        let checkout = root.join(WORKTREE_ROOT).join("repos/owner-repo");
        assert_eq!(
            fs::read_to_string(checkout.join(".fake-fetched"))?,
            "pinned\n"
        );
        Ok(())
    }

    #[test]
    fn missing_base_ref_rejects_merge_subjects() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let _fake_bin_guard = fake_bin_lock();
        write_fake_bin(root, "git", fake_git_script())?;
        let tools = fake_tools(root);
        let case = EvalCase {
            id: "case-a".to_string(),
            repo: "owner/repo".to_string(),
            subject_ref: "merge-subject".to_string(),
            base_ref: None,
            curation: None,
        };

        let error = prepare_case_checkout(root, &case, &tools).unwrap_err();

        assert!(error.to_string().contains("set base_ref explicitly"));
        Ok(())
    }

    #[test]
    fn resolve_skill_exports_skill_refs_and_rejects_ambiguous_sources() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let _fake_bin_guard = fake_bin_lock();
        write_fake_bin(root, "git", fake_git_script())?;
        write_eval_fixture(root)?;
        let tools = fake_tools(root);

        let resolved = resolve_skill(root, Some("main"), None, &tools)?;

        assert_eq!(resolved.source, "git:main");
        assert!(resolved.path.join("SKILL.md").is_file());
        let error = resolve_skill(
            root,
            Some("main"),
            Some(&root.join(DEFAULT_SKILL_PATH)),
            &tools,
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("use only one of --skill-ref or --skill-path")
        );
        Ok(())
    }

    #[test]
    fn collect_run_findings_requires_every_repeat_output_and_namespaces_ids() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let run_dir = temp.path();
        let mut metadata = run_meta("run", "fake-model");
        metadata.repeats = 1;
        write_json(&run_dir.join("run.json"), &metadata)?;
        let mut raw_finding = finding("F1", "first", 1);
        raw_finding.repeat = None;
        write_json(
            &run_dir.join("repeat-1/findings.json"),
            &FindingsFile {
                findings: vec![raw_finding],
            },
        )?;

        let findings = collect_run_findings(run_dir)?;
        assert_eq!(findings[0].id, "repeat-1:F1");
        assert_eq!(findings[0].repeat, Some(1));

        metadata.repeats = 2;
        write_json(&run_dir.join("run.json"), &metadata)?;
        let error = collect_run_findings(run_dir).unwrap_err();
        assert!(error.to_string().contains("missing findings for repeat 2"));
        Ok(())
    }

    #[test]
    fn collect_run_findings_rejects_duplicate_model_ids() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let run_dir = temp.path();
        write_json(&run_dir.join("run.json"), &run_meta("run", "fake-model"))?;
        write_json(
            &run_dir.join("repeat-1/findings.json"),
            &FindingsFile {
                findings: vec![finding("F1", "first", 1), finding("F1", "second", 1)],
            },
        )?;

        let error = collect_run_findings(run_dir).unwrap_err();

        assert!(error.to_string().contains("duplicate finding id"));
        Ok(())
    }

    /// Coordinator output is validated before the repeat is accepted, rather
    /// than leaving a completed run that fails only when a later compare tries
    /// to use model-assigned ids as join keys.
    #[test]
    fn swarm_output_rejects_duplicate_finding_ids_immediately() {
        let findings = vec![finding("F1", "first", 1), finding("F1", "second", 1)];

        let error = ensure_unique_finding_ids(&findings).unwrap_err();

        assert!(error.to_string().contains("duplicate finding id 'F1'"));
    }

    #[test]
    fn run_command_rejects_zero_repeats() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let error = RunCommand {
            skill: DEFAULT_SKILL.to_string(),
            case_id: "case-a".to_string(),
            model: "fake-model".to_string(),
            repeats: 0,
            label: "smoke".to_string(),
            skill_ref: None,
            skill_path: None,
            reviewer: None,
            backend: Backend::Codex,
            effort: None,
        }
        .run_with_tools(root, &fake_tools(root))
        .unwrap_err();

        assert!(error.to_string().contains("--repeats must be at least 1"));
        Ok(())
    }

    #[test]
    fn checked_in_eval_config_parses() -> Result<()> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask should live under repo root");
        let cases = load_cases(root)?;
        let case = cases
            .get("treeward-swapped-fifo")
            .expect("seed case should exist");
        assert_eq!(case.repo, "scode/treeward");
        assert_eq!(case.subject_ref, "code-review-eval-swapped-fifo");
        assert_eq!(case.curation, Some(Curation::Hand));

        for schema in [
            "findings.schema.json",
            "reviewer-findings.schema.json",
            "swarm-result.schema.json",
            "judgments.schema.json",
            "matches.schema.json",
            "suggestions.schema.json",
        ] {
            let path = root.join(EVAL_ROOT).join("schemas").join(schema);
            let value: serde_json::Value = read_json(&path)?;
            assert_eq!(
                value.get("$schema").and_then(|v| v.as_str()),
                Some("https://json-schema.org/draft/2020-12/schema")
            );
            assert_eq!(value.get("type").and_then(|v| v.as_str()), Some("object"));
            assert_eq!(
                value.get("additionalProperties").and_then(|v| v.as_bool()),
                Some(false)
            );
        }

        assert_schema_contract(
            &read_json(&root.join(EVAL_ROOT).join("schemas/findings.schema.json"))?,
            "findings",
            // reviewers is required here even though the Rust struct reads it
            // as optional: OpenAI strict output schemas reject properties
            // missing from `required`, so optionality must live in the reader.
            &[
                "id",
                "category",
                "summary",
                "location",
                "rationale",
                "reviewers",
            ],
            &["id", "category", "summary", "location", "rationale"],
        );
        // The codex-facing reviewer schema has no reviewers property at all:
        // attribution is stamped by the harness after the agent returns, so
        // the agent is never asked (or able) to claim attribution itself.
        assert_schema_contract(
            &read_json(
                &root
                    .join(EVAL_ROOT)
                    .join("schemas/reviewer-findings.schema.json"),
            )?,
            "findings",
            &["id", "category", "summary", "location", "rationale"],
            &["id", "category", "summary", "location", "rationale"],
        );
        let swarm: serde_json::Value = read_json(
            &root
                .join(EVAL_ROOT)
                .join("schemas/swarm-result.schema.json"),
        )?;
        assert_eq!(
            swarm
                .get("required")
                .and_then(|value| value.as_array())
                .map(|values| values
                    .iter()
                    .filter_map(|value| value.as_str())
                    .collect::<Vec<_>>()),
            Some(vec!["findings", "reviewer_execution"])
        );
        assert_eq!(
            swarm
                .pointer("/properties/reviewer_execution/items/required")
                .and_then(|value| value.as_array())
                .map(|values| values
                    .iter()
                    .filter_map(|value| value.as_str())
                    .collect::<Vec<_>>()),
            Some(vec!["reviewer", "status", "passes", "rationale"])
        );
        let judgments: serde_json::Value =
            read_json(&root.join(EVAL_ROOT).join("schemas/judgments.schema.json"))?;
        assert_schema_contract(
            &judgments,
            "judgments",
            &["finding_id", "classification", "rationale"],
            &["finding_id", "rationale"],
        );
        assert_eq!(
            judgments
                .pointer("/properties/judgments/items/properties/classification/enum")
                .and_then(|value| value.as_array())
                .map(|values| values
                    .iter()
                    .filter_map(|value| value.as_str())
                    .collect::<Vec<_>>()),
            Some(vec!["good", "incorrect", "indeterminate"])
        );
        assert_schema_contract(
            &read_json(&root.join(EVAL_ROOT).join("schemas/matches.schema.json"))?,
            "matches",
            &["baseline_finding_id", "candidate_finding_id", "rationale"],
            &["baseline_finding_id", "candidate_finding_id", "rationale"],
        );
        assert_schema_contract(
            &read_json(&root.join(EVAL_ROOT).join("schemas/suggestions.schema.json"))?,
            "suggestions",
            &["summary", "rationale", "target"],
            &["summary", "rationale", "target"],
        );

        Ok(())
    }

    fn assert_schema_contract(
        schema: &serde_json::Value,
        array_key: &str,
        item_required: &[&str],
        string_fields: &[&str],
    ) {
        assert_eq!(
            schema
                .get("required")
                .and_then(|value| value.as_array())
                .map(|values| values
                    .iter()
                    .filter_map(|value| value.as_str())
                    .collect::<Vec<_>>()),
            Some(vec![array_key])
        );
        assert_eq!(
            schema
                .pointer(&format!("/properties/{array_key}/type"))
                .and_then(|value| value.as_str()),
            Some("array")
        );
        assert_eq!(
            schema
                .pointer(&format!("/properties/{array_key}/items/required"))
                .and_then(|value| value.as_array())
                .map(|values| values
                    .iter()
                    .filter_map(|value| value.as_str())
                    .collect::<Vec<_>>()),
            Some(item_required.to_vec())
        );
        for field in string_fields {
            assert_eq!(
                schema
                    .pointer(&format!(
                        "/properties/{array_key}/items/properties/{field}/type"
                    ))
                    .and_then(|value| value.as_str()),
                Some("string")
            );
        }
    }

    #[test]
    fn compare_and_synthesize_use_fake_codex_artifacts() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        write_eval_fixture(root)?;
        let _fake_bin_guard = fake_bin_lock();
        write_fake_bin(root, "git", fake_git_script())?;
        write_fake_bin(root, "codex", fake_codex_script())?;
        let tools = fake_tools(root);

        let baseline = root.join(RUN_ROOT).join("baseline-run");
        let candidate = root.join(RUN_ROOT).join("candidate-run");
        write_json(
            &baseline.join("run.json"),
            &run_meta("baseline-run", "fake-model"),
        )?;
        write_json(
            &candidate.join("run.json"),
            &run_meta("candidate-run", "fake-model"),
        )?;
        write_baseline_marker(&baseline)?;
        write_json(
            &baseline.join("repeat-1/findings.json"),
            &FindingsFile {
                findings: vec![finding("B1", "baseline bug", 1)],
            },
        )?;
        write_json(
            &candidate.join("repeat-1/findings.json"),
            &FindingsFile {
                findings: vec![finding("C1", "candidate bug", 1)],
            },
        )?;

        CompareCommand {
            baseline: baseline.strip_prefix(root)?.to_path_buf(),
            candidate: candidate.strip_prefix(root)?.to_path_buf(),
            model: Some("fake-model".to_string()),
            backend: Backend::Codex,
            effort: None,
        }
        .run_with_tools(root, &tools)?;

        let comparison_path = fs::read_dir(root.join(RUN_ROOT))?
            .filter_map(Result::ok)
            .map(|entry| entry.path().join("comparison.json"))
            .find(|path| path.exists())
            .expect("expected comparison output");
        let comparison: ComparisonFile = read_json(&comparison_path)?;
        assert_eq!(comparison.likely_regressions.len(), 1);

        SynthesizeCommand {
            comparison: comparison_path.strip_prefix(root)?.to_path_buf(),
            model: Some("fake-model".to_string()),
            backend: Backend::Codex,
            effort: None,
        }
        .run_with_tools(root, &tools)?;
        assert!(
            comparison_path
                .parent()
                .unwrap()
                .join("suggestions.json")
                .is_file()
        );

        Ok(())
    }

    #[test]
    fn synthesize_requires_model_when_comparison_has_no_candidate_model() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        write_eval_fixture(root)?;
        let comparison_path = root.join(RUN_ROOT).join("comparison-run/comparison.json");
        write_json(
            &comparison_path,
            &ComparisonFile {
                baseline_run: "baseline-run".to_string(),
                candidate_run: "candidate-run".to_string(),
                case_id: "case".to_string(),
                candidate_model: None,
                baseline_backend: Backend::Codex,
                candidate_backend: Backend::Codex,
                matches: Vec::new(),
                likely_regressions: Vec::new(),
                nondeterminism_notes: Vec::new(),
            },
        )?;

        let error = SynthesizeCommand {
            comparison: comparison_path.strip_prefix(root)?.to_path_buf(),
            model: None,
            backend: Backend::Codex,
            effort: None,
        }
        .run_with_tools(root, &fake_tools(root))
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("--model is required when comparison has no candidate model")
        );
        Ok(())
    }

    #[test]
    fn compare_rejects_missing_baseline_marker() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let baseline = root.join(RUN_ROOT).join("baseline-run");
        let candidate = root.join(RUN_ROOT).join("candidate-run");
        write_json(
            &baseline.join("run.json"),
            &run_meta("baseline-run", "fake-model"),
        )?;
        write_json(
            &candidate.join("run.json"),
            &run_meta("candidate-run", "fake-model"),
        )?;

        let error = CompareCommand {
            baseline: baseline.strip_prefix(root)?.to_path_buf(),
            candidate: candidate.strip_prefix(root)?.to_path_buf(),
            model: Some("fake-model".to_string()),
            backend: Backend::Codex,
            effort: None,
        }
        .run_with_tools(root, &fake_tools(root))
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("has not been marked as a baseline")
        );
        Ok(())
    }

    #[test]
    fn compare_rejects_mismatched_cases_and_diffs() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        write_eval_fixture(root)?;
        let _fake_bin_guard = fake_bin_lock();
        write_fake_bin(root, "git", fake_git_script())?;
        write_fake_bin(root, "codex", fake_codex_script())?;
        let tools = fake_tools(root);
        let baseline = root.join(RUN_ROOT).join("baseline-run");
        let candidate = root.join(RUN_ROOT).join("candidate-run");
        write_baseline_marker(&baseline)?;
        write_json(
            &baseline.join("run.json"),
            &run_meta("baseline-run", "fake-model"),
        )?;

        let mut wrong_case = run_meta("candidate-run", "fake-model");
        wrong_case.case_id = "other-case".to_string();
        write_json(&candidate.join("run.json"), &wrong_case)?;
        let error = CompareCommand {
            baseline: baseline.strip_prefix(root)?.to_path_buf(),
            candidate: candidate.strip_prefix(root)?.to_path_buf(),
            model: Some("fake-model".to_string()),
            backend: Backend::Codex,
            effort: None,
        }
        .run_with_tools(root, &tools)
        .unwrap_err();
        assert!(error.to_string().contains("does not match candidate case"));

        let mut wrong_diff = run_meta("candidate-run", "fake-model");
        wrong_diff.subject_sha = "different-subject".to_string();
        write_json(&candidate.join("run.json"), &wrong_diff)?;
        let error = CompareCommand {
            baseline: baseline.strip_prefix(root)?.to_path_buf(),
            candidate: candidate.strip_prefix(root)?.to_path_buf(),
            model: Some("fake-model".to_string()),
            backend: Backend::Codex,
            effort: None,
        }
        .run_with_tools(root, &tools)
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("must review the same resolved diff")
        );

        // A single-reviewer run and a full-panel run measure different
        // things; comparing them would report the missing panel's findings
        // as regressions.
        let mut wrong_reviewer = run_meta("candidate-run", "fake-model");
        wrong_reviewer.reviewer = Some("test-quality".to_string());
        write_json(&candidate.join("run.json"), &wrong_reviewer)?;
        let error = CompareCommand {
            baseline: baseline.strip_prefix(root)?.to_path_buf(),
            candidate: candidate.strip_prefix(root)?.to_path_buf(),
            model: Some("fake-model".to_string()),
            backend: Backend::Codex,
            effort: None,
        }
        .run_with_tools(root, &tools)
        .unwrap_err();
        assert!(error.to_string().contains("reviewer restriction"));

        // Legacy unrestricted artifacts used harness-owned fan-out. They are
        // not a valid baseline for a run that exercised the candidate
        // coordinator, even though both have no --reviewer restriction.
        let mut wrong_mode = run_meta("candidate-run", "fake-model");
        wrong_mode.execution_mode = ExecutionMode::LegacyPanel;
        write_json(&candidate.join("run.json"), &wrong_mode)?;
        let error = CompareCommand {
            baseline: baseline.strip_prefix(root)?.to_path_buf(),
            candidate: candidate.strip_prefix(root)?.to_path_buf(),
            model: Some("fake-model".to_string()),
            backend: Backend::Codex,
            effort: None,
        }
        .run_with_tools(root, &tools)
        .unwrap_err();
        assert!(error.to_string().contains("execution mode"));

        // Effort changes how hard the subject agents work, so runs at
        // different efforts measure different things, exactly like the
        // reviewer restriction.
        let mut wrong_effort = run_meta("candidate-run", "fake-model");
        wrong_effort.effort = Some("high".to_string());
        write_json(&candidate.join("run.json"), &wrong_effort)?;
        let error = CompareCommand {
            baseline: baseline.strip_prefix(root)?.to_path_buf(),
            candidate: candidate.strip_prefix(root)?.to_path_buf(),
            model: Some("fake-model".to_string()),
            backend: Backend::Codex,
            effort: None,
        }
        .run_with_tools(root, &tools)
        .unwrap_err();
        assert!(error.to_string().contains("baseline effort"));
        Ok(())
    }

    /// Restricted artifacts written before execution modes existed used the
    /// same direct-charter workflow as current `Reviewer` runs. They remain
    /// valid baselines even though serde fills their missing mode with the
    /// unrestricted legacy default.
    #[test]
    fn legacy_restricted_runs_compare_as_reviewer_mode() -> Result<()> {
        let mut legacy = run_meta("legacy", "fake-model");
        legacy.reviewer = Some("test-quality".to_string());
        let mut serialized = serde_json::to_value(&legacy)?;
        serialized
            .as_object_mut()
            .expect("run metadata serializes as an object")
            .remove("execution_mode");
        let legacy: RunMetadata = serde_json::from_value(serialized)?;
        let mut current = run_meta("current", "fake-model");
        current.reviewer = Some("test-quality".to_string());
        current.execution_mode = ExecutionMode::Reviewer;

        assert_eq!(
            legacy.comparison_execution_mode(),
            current.comparison_execution_mode()
        );
        assert_eq!(legacy.comparison_execution_mode(), ExecutionMode::Reviewer);
        Ok(())
    }

    #[test]
    fn nondeterminism_notes_report_findings_seen_in_some_repeats() {
        let notes = nondeterminism_notes(
            "baseline",
            &[
                finding("F1", "flaky finding", 1),
                finding("F2", "stable context", 2),
            ],
        );

        assert!(notes.iter().any(|note| {
            note.contains("baseline finding appeared in 1/2 repeats")
                && note.contains("flaky finding")
        }));
    }

    #[test]
    fn validates_judgment_and_match_ids() {
        let baseline = vec![finding("B1", "baseline bug", 1)];

        let missing = validate_judgments("baseline", &baseline, &[]);
        assert!(
            missing
                .unwrap_err()
                .to_string()
                .contains("omitted judgment")
        );

        let unknown = validate_judgments(
            "baseline",
            &baseline,
            &[judgment("repeat-1:unknown", Classification::Good)],
        );
        assert!(
            unknown
                .unwrap_err()
                .to_string()
                .contains("unknown finding id")
        );

        let duplicate = validate_judgments(
            "baseline",
            &[finding("repeat-1:B1", "baseline bug", 1)],
            &[
                judgment("repeat-1:B1", Classification::Good),
                judgment("repeat-1:B1", Classification::Incorrect),
            ],
        );
        assert!(
            duplicate
                .unwrap_err()
                .to_string()
                .contains("duplicate judgment")
        );

        let bad_match = validate_matches(
            &[finding("repeat-1:B1", "baseline bug", 1)],
            &[finding("repeat-1:C1", "candidate bug", 1)],
            &[FindingMatch {
                baseline_finding_id: "repeat-1:B1".to_string(),
                candidate_finding_id: "repeat-1:unknown".to_string(),
                rationale: "same issue".to_string(),
            }],
        );
        assert!(
            bad_match
                .unwrap_err()
                .to_string()
                .contains("unknown candidate finding id")
        );

        let bad_match = validate_matches(
            &[finding("repeat-1:B1", "baseline bug", 1)],
            &[finding("repeat-1:C1", "candidate bug", 1)],
            &[FindingMatch {
                baseline_finding_id: "repeat-1:unknown".to_string(),
                candidate_finding_id: "repeat-1:C1".to_string(),
                rationale: "same issue".to_string(),
            }],
        );
        assert!(
            bad_match
                .unwrap_err()
                .to_string()
                .contains("unknown baseline finding id")
        );

        let duplicate_baseline = validate_matches(
            &[finding("repeat-1:B1", "baseline bug", 1)],
            &[
                finding("repeat-1:C1", "candidate bug", 1),
                finding("repeat-1:C2", "candidate bug", 1),
            ],
            &[
                FindingMatch {
                    baseline_finding_id: "repeat-1:B1".to_string(),
                    candidate_finding_id: "repeat-1:C1".to_string(),
                    rationale: "same issue".to_string(),
                },
                FindingMatch {
                    baseline_finding_id: "repeat-1:B1".to_string(),
                    candidate_finding_id: "repeat-1:C2".to_string(),
                    rationale: "same issue".to_string(),
                },
            ],
        );
        assert!(
            duplicate_baseline
                .unwrap_err()
                .to_string()
                .contains("duplicate baseline finding id")
        );

        let duplicate_candidate = validate_matches(
            &[
                finding("repeat-1:B1", "baseline bug", 1),
                finding("repeat-1:B2", "baseline bug", 1),
            ],
            &[finding("repeat-1:C1", "candidate bug", 1)],
            &[
                FindingMatch {
                    baseline_finding_id: "repeat-1:B1".to_string(),
                    candidate_finding_id: "repeat-1:C1".to_string(),
                    rationale: "same issue".to_string(),
                },
                FindingMatch {
                    baseline_finding_id: "repeat-1:B2".to_string(),
                    candidate_finding_id: "repeat-1:C1".to_string(),
                    rationale: "same issue".to_string(),
                },
            ],
        );
        assert!(
            duplicate_candidate
                .unwrap_err()
                .to_string()
                .contains("duplicate candidate finding id")
        );
    }

    fn run_meta(label: &str, model: &str) -> RunMetadata {
        RunMetadata {
            id: label.to_string(),
            skill: DEFAULT_SKILL.to_string(),
            label: label.to_string(),
            model: model.to_string(),
            repeats: 1,
            case_id: "case".to_string(),
            repo: "owner/repo".to_string(),
            subject_ref: "subject".to_string(),
            subject_sha: "subject".to_string(),
            base_ref: "base".to_string(),
            base_sha: "base".to_string(),
            curation: None,
            reviewer: None,
            execution_mode: ExecutionMode::Swarm,
            backend: Backend::Codex,
            effort: None,
            skill_source: "working-tree".to_string(),
            skill_path: DEFAULT_SKILL_PATH.to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn finding(id: &str, summary: &str, repeat: usize) -> Finding {
        Finding {
            id: id.to_string(),
            category: "correctness".to_string(),
            summary: summary.to_string(),
            location: "src/lib.rs:1".to_string(),
            rationale: "because".to_string(),
            reviewers: None,
            repeat: Some(repeat),
        }
    }

    /// The run command re-serializes findings.json through the Finding
    /// struct, so any field the struct does not declare is silently stripped
    /// from the stored artifact. This pins reviewer attribution surviving
    /// that round trip: losing it would quietly destroy the only record of
    /// which reviewer charters surfaced each finding, which is the data a
    /// per-reviewer contribution analysis depends on.
    #[test]
    fn finding_round_trip_preserves_reviewer_attribution() -> Result<()> {
        let raw = r#"{
            "findings": [{
                "id": "F1",
                "category": "correctness",
                "summary": "s",
                "location": "src/lib.rs:1",
                "rationale": "r",
                "reviewers": ["correctness-general", "correctness-data-flow"]
            }]
        }"#;
        let file: FindingsFile = serde_json::from_str(raw)?;
        let round_tripped = serde_json::to_string(&file)?;
        let reparsed: FindingsFile = serde_json::from_str(&round_tripped)?;
        assert_eq!(
            reparsed.findings[0].reviewers.as_deref(),
            Some(
                &[
                    "correctness-general".to_string(),
                    "correctness-data-flow".to_string()
                ][..]
            )
        );
        Ok(())
    }

    fn judgment(id: &str, classification: Classification) -> Judgment {
        Judgment {
            finding_id: id.to_string(),
            classification,
            rationale: "because".to_string(),
        }
    }

    fn write_eval_fixture(root: &Path) -> Result<()> {
        fs::create_dir_all(root.join(EVAL_ROOT).join("prompts"))?;
        fs::create_dir_all(root.join(EVAL_ROOT).join("schemas"))?;
        fs::create_dir_all(root.join(DEFAULT_SKILL_PATH))?;
        fs::write(
            root.join(EVAL_ROOT).join("cases.toml"),
            r#"
[[cases]]
id = "case-a"
repo = "owner/repo"
subject_ref = "subject"
curation = "hand"

[[cases]]
id = "case-mined"
repo = "owner/repo"
subject_ref = "subject"
curation = "mined"
"#,
        )?;
        fs::write(
            root.join(EVAL_ROOT).join("prompts/reviewer.md"),
            "review {{charter_path}} in {{repo_path}} scope {{scope_path}} \
             range {{base_sha}}..{{subject_sha}}",
        )?;
        fs::write(
            root.join(EVAL_ROOT).join("prompts/swarm.md"),
            "Follow candidate skill {{skill_path}} as coordinator in {{repo_path}} \
             with {{scope_path}} labeled {{scope_label}}",
        )?;
        fs::write(
            root.join(EVAL_ROOT).join("prompts/preflight.md"),
            "preflight {{charter_path}} scope {{scope_path}}",
        )?;
        fs::write(root.join(EVAL_ROOT).join("prompts/judge.md"), "judge")?;
        fs::write(root.join(EVAL_ROOT).join("prompts/match.md"), "match")?;
        fs::write(
            root.join(EVAL_ROOT).join("prompts/synthesize.md"),
            "synthesize",
        )?;
        for schema in [
            "findings.schema.json",
            "reviewer-findings.schema.json",
            "swarm-result.schema.json",
            "judgments.schema.json",
            "matches.schema.json",
            "suggestions.schema.json",
        ] {
            fs::write(root.join(EVAL_ROOT).join("schemas").join(schema), "{}")?;
        }
        fs::write(root.join(DEFAULT_SKILL_PATH).join("SKILL.md"), "# skill")?;
        // A fixture panel exercising every discovery rule: two spawnable
        // charters, one shared base charter (skipped), and one SPEC.md-gated
        // charter (skipped because the fake checkout has no SPEC.md).
        fs::create_dir_all(root.join(DEFAULT_SKILL_PATH).join("reviewers"))?;
        fs::write(
            root.join(DEFAULT_SKILL_PATH)
                .join("reviewers/test-quality.md"),
            "# test-quality-reviewer",
        )?;
        fs::write(
            root.join(DEFAULT_SKILL_PATH)
                .join("reviewers/docs-comments.md"),
            "# docs-comments-reviewer",
        )?;
        fs::write(
            root.join(DEFAULT_SKILL_PATH)
                .join("reviewers/correctness.md"),
            "# Correctness base charter\n\nNOTE: This file is not spawned as a reviewer on its own.",
        )?;
        fs::write(
            root.join(DEFAULT_SKILL_PATH)
                .join("reviewers/spec-compliance.md"),
            "# spec-compliance-reviewer\n\n_(Only spawned when `SPEC.md` exists at the project root.)_",
        )?;
        Ok(())
    }

    fn write_baseline_marker(run_dir: &Path) -> Result<()> {
        write_json(
            &run_dir.join("baseline.json"),
            &BaselineMarker {
                run_id: "baseline-run".to_string(),
                case_id: "case".to_string(),
                model: "fake-model".to_string(),
                skill_source: "working-tree".to_string(),
                marked_at: "2026-01-01T00:00:00Z".to_string(),
            },
        )
    }

    /// Serializes every test that writes fake executables and then spawns
    /// them. The test binary's threads fork constantly; a fork that lands
    /// between another test's write and exec of its fake bin briefly holds
    /// the (CLOEXEC, but not yet execed) write fd open in the child, and the
    /// exec then fails with ETXTBSY. Observed as rare cross-suite "one test
    /// randomly failed" flakes whose error came from a stage the failing
    /// test's assertions never touch. Holding this lock for the duration of
    /// each write-then-spawn test closes the window; the serialized tests
    /// finish in well under a second combined.
    fn fake_bin_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn write_fake_bin(root: &Path, name: &str, script: &str) -> Result<()> {
        let bin = root.join("bin");
        fs::create_dir_all(&bin)?;
        let path = bin.join(name);
        fs::write(&path, script)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&path, perms)?;
        }
        Ok(())
    }

    fn fake_tools(root: &Path) -> ToolEnv {
        ToolEnv {
            git: root.join("bin/git"),
            codex: root.join("bin/codex"),
            claude: root.join("bin/claude"),
        }
    }

    fn fake_git_script() -> &'static str {
        r#"#!/usr/bin/env bash
set -euo pipefail
if [ "${1:-}" = "clone" ]; then
  mkdir -p "${4}/.git"
  exit 0
fi
repo=""
if [ "${1:-}" = "-C" ]; then
  repo="$2"
  shift 2
fi
case "$1" in
  archive)
    tmp="$(mktemp -d)"
    mkdir -p "$tmp/agent-skills/pre-pr-review-swarm"
    printf '# exported skill\n' > "$tmp/agent-skills/pre-pr-review-swarm/SKILL.md"
    tar -C "$tmp" -cf - agent-skills/pre-pr-review-swarm
    rm -rf "$tmp"
    ;;
  fetch)
    # Record explicit-ref fetches ("fetch origin <ref>") so tests can assert
    # the fetch fallback fired; the routine "fetch --tags origin" is ignored.
    if [ "${2:-}" = "origin" ] && [ -n "${3:-}" ] && [ -n "$repo" ]; then
      printf '%s\n' "$3" > "$repo/.fake-fetched"
    fi
    exit 0
    ;;
  checkout)
    mkdir -p "$repo/.git"
    printf '%s\n' "${@: -1}" > "$repo/.fake-head-ref"
    ;;
  rev-parse)
    if [ "${2:-}" = "--verify" ]; then
      ref="${@: -1}"
      case "$ref" in
        refs/remotes/origin/subject^\{commit\}) echo subject-remote-sha ;;
        refs/remotes/origin/explicit-base^\{commit\}) echo explicit-base-remote-sha ;;
        refs/remotes/origin/base^\{commit\}) echo base-sha ;;
        *) exit 1 ;;
      esac
    elif [ "$2" = "HEAD" ]; then
      if [ -n "$repo" ] && [ -f "$repo/.fake-head-ref" ]; then
        cat "$repo/.fake-head-ref"
      else
        echo subject-sha
      fi
    else
      case "$2" in
        subject^\{commit\}) echo subject-local-stale-sha ;;
        explicit-base^\{commit\}) echo explicit-base-local-stale-sha ;;
        base^\{commit\}) echo base-sha ;;
        pinned^\{commit\})
          # Simulates a commit reachable only via fetch-by-SHA: resolvable
          # only after an explicit "fetch origin pinned" recorded the marker.
          if [ -n "$repo" ] && [ -f "$repo/.fake-fetched" ]; then
            echo pinned-fetched-sha
          else
            exit 1
          fi
          ;;
        *) echo "$2" ;;
      esac
    fi
    ;;
  show)
    head_ref=""
    if [ -n "$repo" ] && [ -f "$repo/.fake-head-ref" ]; then
      head_ref="$(cat "$repo/.fake-head-ref")"
    fi
    if [[ "$head_ref" == merge-subject* ]]; then
      echo "base-sha other-parent"
    else
      echo base-sha
    fi
    ;;
  diff)
    echo "diff --git a/src/lib.rs b/src/lib.rs"
    ;;
  *)
    echo "unexpected fake git args: $*" >&2
    exit 1
    ;;
esac
"#
    }

    fn fake_codex_script() -> &'static str {
        r#"#!/usr/bin/env bash
set -euo pipefail
out=""
schema=""
cwd=""
config=""
saw_ephemeral=0
saw_sandbox_bypass=0
saw_ignore_user_config=0
saw_ignore_rules=0
stdin_prompt=""
is_swarm=0
while [ "$#" -gt 0 ]; do
  if [ "$1" = "-o" ]; then
    out="$2"
    shift 2
  elif [ "$1" = "--output-schema" ]; then
    schema="$2"
    shift 2
  elif [ "$1" = "-c" ]; then
    config="${config}${2}"$'\n'
    shift 2
  elif [ "$1" = "--ephemeral" ]; then
    saw_ephemeral=1
    shift
  elif [ "$1" = "--ignore-user-config" ]; then
    saw_ignore_user_config=1
    shift
  elif [ "$1" = "--ignore-rules" ]; then
    saw_ignore_rules=1
    shift
  elif [ "$1" = "--dangerously-bypass-approvals-and-sandbox" ]; then
    # TEMPORARY: mirrors the sandbox bypass in run_codex_json; assert the
    # sandboxed form again once sandboxing is re-enabled.
    saw_sandbox_bypass=1
    shift
  elif [ "$1" = "-s" ]; then
    echo "fake codex saw a sandbox flag while sandboxing is disabled" >&2
    exit 2
  elif [ "$1" = "-C" ]; then
    cwd="$2"
    shift 2
  elif [ "$1" = "-a" ] || [ "$1" = "--ask-for-approval" ]; then
    echo "fake codex rejects approval flags" >&2
    exit 2
  else
    shift
  fi
done
stdin_prompt="$(cat)"
test "$saw_ephemeral" = "1"
test "$saw_sandbox_bypass" = "1"
test "$saw_ignore_user_config" = "1"
test "$saw_ignore_rules" = "1"
test -n "$cwd"
test -n "$stdin_prompt"
mkdir -p "$(dirname "$out")"
# Record -c overrides next to the output so tests can assert what config
# (e.g. model_reasoning_effort) actually reached the codex command line.
if [ -n "$config" ]; then
  printf '%s' "$config" >"$out.config"
fi
case "$(basename "$schema")" in
  swarm-result.schema.json)
    is_swarm=1
    grep -F "Follow candidate skill" <<<"$stdin_prompt" >/dev/null
    grep -F "owner-repo" <<<"$stdin_prompt" >/dev/null
    cat >"$out" <<'JSON'
{
  "findings": [
    {
      "id": "F1",
      "category": "correctness",
      "summary": "example finding",
      "location": "src/lib.rs:1",
      "rationale": "example rationale",
      "reviewers": ["docs-comments"]
    }
  ],
  "reviewer_execution": [
    {
      "reviewer": "docs-comments",
      "status": "completed",
      "passes": 2,
      "rationale": ""
    },
    {
      "reviewer": "test-quality",
      "status": "completed",
      "passes": 1,
      "rationale": ""
    },
    {
      "reviewer": "spec-compliance",
      "status": "skipped",
      "passes": 0,
      "rationale": "target has no SPEC.md"
    }
  ]
}
JSON
    ;;
  judgments.schema.json)
    grep -F "Review input:" <<<"$stdin_prompt" >/dev/null
    grep -F "diff --git" <<<"$stdin_prompt" >/dev/null
    grep -F "findings" <<<"$stdin_prompt" >/dev/null
    case "$(basename "$out")" in
      baseline-judgments.json)
        cat >"$out" <<'JSON'
{
  "judgments": [
    { "finding_id": "repeat-1:B1", "classification": "good", "rationale": "real" }
  ]
}
JSON
        ;;
      candidate-judgments.json)
        cat >"$out" <<'JSON'
{
  "judgments": [
    { "finding_id": "repeat-1:C1", "classification": "good", "rationale": "real" }
  ]
}
JSON
        ;;
      *)
        cat >"$out" <<'JSON'
{
  "judgments": [
    { "finding_id": "repeat-1:B1", "classification": "good", "rationale": "real" },
    { "finding_id": "repeat-1:C1", "classification": "good", "rationale": "real" }
  ]
}
JSON
        ;;
    esac
    ;;
  matches.schema.json)
    cat >"$out" <<'JSON'
{ "matches": [] }
JSON
    ;;
  suggestions.schema.json)
    cat >"$out" <<'JSON'
{
  "suggestions": [
    { "summary": "tighten reviewer charter", "rationale": "lost finding", "target": "reviewers/correctness.md" }
  ]
}
JSON
    ;;
  *)
    case "$(basename "$out")" in
      preflight-*.findings.json)
        cat >"$out" <<'JSON'
{
  "findings": [
    {
      "id": "P1",
      "category": "correctness",
      "summary": "planted parity bug",
      "location": "src/even.rs:3",
      "rationale": "n % 2 == 1 tests oddness, not evenness"
    }
  ]
}
JSON
        ;;
      *)
        grep -F "pre-pr-review-swarm" <<<"$stdin_prompt" >/dev/null
        grep -F "owner-repo" <<<"$stdin_prompt" >/dev/null
        grep -F "base-sha..subject-remote-sha" <<<"$stdin_prompt" >/dev/null
        printf '%s' "$stdin_prompt" >"$out.prompt"
        cat >"$out" <<'JSON'
{
  "findings": [
    {
      "id": "F1",
      "category": "correctness",
      "summary": "example finding",
      "location": "src/lib.rs:1",
      "rationale": "example rationale"
    }
  ]
}
JSON
        ;;
    esac
    ;;
esac
# Mimic the real --json event stream closely enough for post-run
# verification: one completed command, a turn.completed with usage, and an
# unknown trailing event that digesting must tolerate.
echo '{"type":"item.completed","item":{"id":"item_1","type":"command_execution","command":"true","status":"completed"}}'
if [ "$is_swarm" = "1" ]; then
  echo '{"type":"item.completed","item":{"id":"spawn_1","type":"collab_tool_call","tool":"spawn_agent","receiver_thread_ids":["reviewer-1"],"status":"completed"}}'
  echo '{"type":"item.completed","item":{"id":"spawn_2","type":"collab_tool_call","tool":"spawn_agent","receiver_thread_ids":["reviewer-2"],"status":"completed"}}'
  echo '{"type":"item.completed","item":{"id":"followup_1","type":"collab_tool_call","tool":"send_input","receiver_thread_ids":["reviewer-1"],"status":"completed"}}'
fi
echo '{"type":"turn.completed","usage":{"input_tokens":100,"cached_input_tokens":0,"output_tokens":42,"reasoning_output_tokens":5}}'
echo '{"event":"done"}'
"#
    }

    /// A fake `claude` emitting a realistic `stream-json` transcript: an init
    /// event, a thinking block, a Bash `tool_use` (real work), a tool_result,
    /// the enforced `StructuredOutput` tool_use, and a terminal `result` event
    /// carrying `structured_output` and `usage`.
    ///
    /// Unlike the codex fake, this cannot branch on the schema basename: claude
    /// receives `--json-schema` as literal contents (and the fixture writes
    /// identical `{}` for every schema), and there is no `-o` output path to
    /// key on either. Preflight and reviewer runs even share one schema, so the
    /// only signal that distinguishes every invocation is the prompt. This fake
    /// therefore branches on prompt content, which cleanly separates preflight,
    /// reviewer, judge, matcher, and synthesis calls.
    fn fake_claude_script() -> &'static str {
        r#"#!/usr/bin/env bash
set -euo pipefail
model=""
effort=""
schema=""
saw_safe_mode=0
saw_skip_perms=0
saw_stream_json=0
saw_no_session=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    --model) model="$2"; shift 2 ;;
    --effort) effort="$2"; shift 2 ;;
    --json-schema) schema="$2"; shift 2 ;;
    --output-format)
      test "$2" = "stream-json"
      saw_stream_json=1
      shift 2
      ;;
    --safe-mode) saw_safe_mode=1; shift ;;
    --dangerously-skip-permissions) saw_skip_perms=1; shift ;;
    --no-session-persistence) saw_no_session=1; shift ;;
    --bare)
      # --bare forces API-key-only auth; the harness must never use it.
      echo "fake claude rejects --bare" >&2
      exit 2
      ;;
    -C)
      # claude has no -C; cwd must arrive via the child working directory.
      echo "fake claude rejects -C" >&2
      exit 2
      ;;
    *) shift ;;
  esac
done
prompt="$(cat)"
# Every isolation and format flag the harness promises must be present, and
# the print-mode background ceiling must be raised to the finite one-hour
# value on the child env (not 0/unlimited — a wedged child must not be able
# to hang an unattended eval forever).
test "$saw_safe_mode" = "1"
test "$saw_skip_perms" = "1"
test "$saw_stream_json" = "1"
test "$saw_no_session" = "1"
test -n "$schema"
test -n "$model"
test -n "$prompt"
test "${CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS:-}" = "3600000"

# Emit a full stream whose final message is the given structured_output object.
# The digest must count exactly the one Bash tool_use (StructuredOutput is
# excluded) and read output_tokens=42 from the result event. The init event
# records the model and effort that actually reached the command line — claude
# has no -o sidecar to key a .config file on, so the transcript doubles as the
# invocation evidence tests assert against (an implementation that drops
# --effort must fail, mirroring effort_reaches_codex_config_and_metadata).
emit() {
  local structured="$1"
  echo '{"type":"system","subtype":"init","cwd":"'"$PWD"'","fake_model":"'"$model"'","fake_effort":"'"$effort"'"}'
  echo '{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"considering"}]}}'
  echo '{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Bash","input":{"command":"true"}}]}}'
  echo '{"type":"user","message":{"content":[{"type":"tool_result","content":"ok"}]}}'
  echo '{"type":"assistant","message":{"content":[{"type":"tool_use","name":"StructuredOutput","input":'"$structured"'}]}}'
  echo '{"type":"result","subtype":"success","is_error":false,"num_turns":4,"result":"done","structured_output":'"$structured"',"usage":{"input_tokens":100,"output_tokens":42}}'
}

emit_swarm() {
  local structured='{"findings":[{"id":"F1","category":"correctness","summary":"example finding","location":"src/lib.rs:1","rationale":"example rationale","reviewers":["docs-comments"]}],"reviewer_execution":[{"reviewer":"docs-comments","status":"completed","passes":2,"rationale":""},{"reviewer":"test-quality","status":"completed","passes":1,"rationale":""},{"reviewer":"spec-compliance","status":"skipped","passes":0,"rationale":"target has no SPEC.md"}]}'
  echo '{"type":"system","subtype":"init","cwd":"'"$PWD"'","fake_model":"'"$model"'","fake_effort":"'"$effort"'"}'
  echo '{"type":"assistant","message":{"content":[{"type":"tool_use","id":"agent-docs","name":"Agent","input":{"name":"docs-comments"}}]}}'
  echo '{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"agent-docs","content":"agent id: docs","is_error":false}]}}'
  echo '{"type":"assistant","message":{"content":[{"type":"tool_use","id":"agent-tests","name":"Agent","input":{"name":"test-quality"}}]}}'
  echo '{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"agent-tests","content":"agent id: tests","is_error":false}]}}'
  echo '{"type":"assistant","message":{"content":[{"type":"tool_use","id":"followup-docs","name":"SendMessage","input":{"recipient":"docs-comments"}}]}}'
  echo '{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"followup-docs","content":"sent","is_error":false}]}}'
  echo '{"type":"assistant","message":{"content":[{"type":"tool_use","name":"StructuredOutput","input":'"$structured"'}]}}'
  echo '{"type":"result","subtype":"success","is_error":false,"num_turns":8,"result":"done","structured_output":'"$structured"',"usage":{"input_tokens":100,"output_tokens":42}}'
}

if grep -F "as coordinator" <<<"$prompt" >/dev/null; then
  emit_swarm
elif grep -F "preflight" <<<"$prompt" >/dev/null; then
  emit '{"findings":[{"id":"P1","category":"correctness","summary":"planted parity bug","location":"src/even.rs:3","rationale":"n % 2 == 1 tests oddness, not evenness"}]}'
elif grep -F "Review input:" <<<"$prompt" >/dev/null; then
  # The codex fake keys the baseline/candidate judge split on the -o basename;
  # claude has no -o, but each judge's prompt embeds only its own run's
  # findings, so the ids present in the prompt identify which judge this is.
  # A judge returning ids it was not asked about fails the harness's
  # unknown-finding-id check.
  if grep -F '"repeat-1:B1"' <<<"$prompt" >/dev/null; then
    emit '{"judgments":[{"finding_id":"repeat-1:B1","classification":"good","rationale":"real"}]}'
  else
    emit '{"judgments":[{"finding_id":"repeat-1:C1","classification":"good","rationale":"real"}]}'
  fi
elif grep -F "Match input:" <<<"$prompt" >/dev/null; then
  emit '{"matches":[]}'
elif grep -F "Comparison input:" <<<"$prompt" >/dev/null; then
  emit '{"suggestions":[{"summary":"tighten reviewer charter","rationale":"lost finding","target":"reviewers/correctness.md"}]}'
else
  grep -F "pre-pr-review-swarm" <<<"$prompt" >/dev/null
  grep -F "owner-repo" <<<"$prompt" >/dev/null
  grep -F "base-sha..subject-remote-sha" <<<"$prompt" >/dev/null
  emit '{"findings":[{"id":"F1","category":"correctness","summary":"example finding","location":"src/lib.rs:1","rationale":"example rationale"}]}'
fi
"#
    }

    /// The claude backend must produce the same coordinator-owned artifact
    /// tree as codex, and the verification digest must understand Claude's
    /// native Agent/SendMessage events. This is the claude analog of
    /// `run_command_writes_artifacts_with_fake_git_and_codex`.
    #[test]
    fn run_command_writes_artifacts_on_claude_backend() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        write_eval_fixture(root)?;
        let _fake_bin_guard = fake_bin_lock();
        write_fake_bin(root, "git", fake_git_script())?;
        write_fake_bin(root, "claude", fake_claude_script())?;
        let tools = fake_tools(root);

        RunCommand {
            skill: DEFAULT_SKILL.to_string(),
            case_id: "case-a".to_string(),
            model: "fake-model".to_string(),
            repeats: 2,
            label: "smoke".to_string(),
            skill_ref: None,
            skill_path: Some(root.join(DEFAULT_SKILL_PATH)),
            reviewer: None,
            backend: Backend::Claude,
            effort: Some("high".to_string()),
        }
        .run_with_tools(root, &tools)?;

        let run_dir = fs::read_dir(root.join(RUN_ROOT))?
            .next()
            .expect("expected run dir")?
            .path();
        let metadata: RunMetadata = read_json(&run_dir.join("run.json"))?;
        assert_eq!(metadata.backend, Backend::Claude);
        assert_eq!(metadata.effort.as_deref(), Some("high"));
        // The invocation itself must carry the effort, not just run.json: the
        // fake stamps what it received into the init event, so a
        // run_claude_json that drops --effort fails here even though the
        // metadata (copied from the command struct) would still look right.
        let transcript = fs::read_to_string(run_dir.join("repeat-1/transcript.jsonl"))?;
        assert!(
            transcript.contains("\"fake_effort\":\"high\""),
            "effort did not reach the claude command line: {transcript}"
        );
        assert!(transcript.contains("\"fake_model\":\"fake-model\""));
        for repeat in ["repeat-1", "repeat-2"] {
            assert!(run_dir.join(repeat).join("swarm-result.json").is_file());
            assert!(run_dir.join(repeat).join("transcript.jsonl").is_file());
        }
        // The Agent and SendMessage events are coordinator activity, so the
        // backend-aware digest reports three non-StructuredOutput calls.
        let verification: RunVerification = read_json(&run_dir.join("verification.json"))?;
        assert_eq!(verification.status, "clean");
        assert_eq!(verification.anomaly_count, 0);
        for repeat in &verification.repeats {
            let coordinator = repeat.coordinator.as_ref().unwrap();
            assert_eq!(coordinator.output_tokens, Some(42));
            assert_eq!(coordinator.commands, 3);
            assert_eq!(coordinator.spawned_agents, 2);
            assert_eq!(coordinator.followups, 1);
            assert!(coordinator.anomalies.is_empty());
        }
        Ok(())
    }

    /// claude's `--json-schema` validator cannot resolve the draft-2020-12
    /// `$schema` meta-reference every checked-in schema file declares, and
    /// rejects the whole schema over it — the first real claude smoke run
    /// failed preflight exactly this way. The claude path must strip the key
    /// from what it hands to the CLI while preserving the rest of the schema
    /// and leaving the on-disk file untouched.
    #[test]
    fn claude_schema_contents_strips_meta_schema_key() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("schema.json");
        let original = r#"{
          "$schema": "https://json-schema.org/draft/2020-12/schema",
          "type": "object",
          "required": ["findings"]
        }"#;
        fs::write(&path, original)?;
        let contents = claude_schema_contents(&path)?;
        assert!(!contents.contains("$schema"), "got: {contents}");
        assert!(contents.contains("\"required\""), "got: {contents}");
        assert_eq!(fs::read_to_string(&path)?, original);
        Ok(())
    }

    /// The claude digest must read output tokens and the tool-call count from
    /// claude's own stream shapes, and flag the claude-native signatures of a
    /// run that did nothing. It counts every tool_use except `StructuredOutput`
    /// (so file-access tools like Read/Grep count as real work), which is a
    /// deliberate semantic difference from the codex command-execution count.
    #[test]
    fn claude_transcript_digest_reads_tokens_and_flags_empty_work() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let dir = temp.path();

        let (tokens, commands, anomalies) =
            digest_transcript(Backend::Claude, &dir.join("missing.jsonl"));
        assert_eq!((tokens, commands), (None, 0));
        assert_eq!(anomalies.len(), 1);
        assert!(anomalies[0].contains("missing"));

        let no_result = dir.join("no-result.jsonl");
        fs::write(
            &no_result,
            "{\"type\":\"system\",\"subtype\":\"init\"}\nnot-json\n",
        )?;
        let (tokens, _, anomalies) = digest_transcript(Backend::Claude, &no_result);
        assert_eq!(tokens, None);
        assert!(anomalies.iter().any(|a| a.contains("no result event")));

        let errored = dir.join("errored.jsonl");
        fs::write(
            &errored,
            "{\"type\":\"result\",\"is_error\":true,\"usage\":{\"output_tokens\":9}}\n",
        )?;
        let (tokens, _, anomalies) = digest_transcript(Backend::Claude, &errored);
        assert_eq!(tokens, Some(9));
        assert!(anomalies.iter().any(|a| a.contains("is_error")));

        // An error subtype with is_error unset must also flag: the runtime
        // failure check treats the two signals independently, and the digest
        // must not call a transcript clean that the runner would have rejected.
        let subtype_error = dir.join("subtype-error.jsonl");
        fs::write(
            &subtype_error,
            "{\"type\":\"result\",\"subtype\":\"error_max_turns\",\"usage\":{\"output_tokens\":9}}\n",
        )?;
        let (tokens, _, anomalies) = digest_transcript(Backend::Claude, &subtype_error);
        assert_eq!(tokens, Some(9));
        assert!(
            anomalies
                .iter()
                .any(|a| a.contains("error subtype 'error_max_turns'")),
            "got: {anomalies:?}"
        );

        let no_tokens = dir.join("no-tokens.jsonl");
        fs::write(&no_tokens, "{\"type\":\"result\",\"is_error\":false}\n")?;
        let (tokens, _, anomalies) = digest_transcript(Backend::Claude, &no_tokens);
        assert_eq!(tokens, None);
        assert!(anomalies.iter().any(|a| a.contains("no output tokens")));

        let zero = dir.join("zero.jsonl");
        fs::write(
            &zero,
            "{\"type\":\"result\",\"is_error\":false,\"usage\":{\"output_tokens\":0}}\n",
        )?;
        let (tokens, _, anomalies) = digest_transcript(Backend::Claude, &zero);
        assert_eq!(tokens, Some(0));
        assert!(anomalies.iter().any(|a| a.contains("zero output tokens")));

        // A success result with tokens but no structured_output is exactly the
        // shape run_claude_json refuses ("produced no structured output"), so
        // the digest must not call it clean either — runtime/digest parity is
        // the digest's documented invariant.
        let no_structured = dir.join("no-structured.jsonl");
        fs::write(
            &no_structured,
            "{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"usage\":{\"output_tokens\":7}}\n",
        )?;
        let (tokens, _, anomalies) = digest_transcript(Backend::Claude, &no_structured);
        assert_eq!(tokens, Some(7));
        assert!(
            anomalies.iter().any(|a| a.contains("no structured output")),
            "got: {anomalies:?}"
        );

        // Healthy: a Bash tool_use and a StructuredOutput tool_use across two
        // assistant events; only the Bash one counts.
        let healthy = dir.join("healthy.jsonl");
        fs::write(
            &healthy,
            "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"tool_use\",\"name\":\"Bash\"}]}}\n\
             {\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"tool_use\",\"name\":\"StructuredOutput\"}]}}\n\
             {\"type\":\"result\",\"is_error\":false,\"structured_output\":{\"findings\":[]},\"usage\":{\"output_tokens\":7}}\n",
        )?;
        let (tokens, commands, anomalies) = digest_transcript(Backend::Claude, &healthy);
        assert_eq!((tokens, commands), (Some(7), 1));
        assert!(anomalies.is_empty(), "got: {anomalies:?}");
        Ok(())
    }

    /// A failed claude run must surface the cause from the terminal `result`
    /// event — claude, like codex, reports failures inside the stream rather
    /// than on stderr — and name the transcript path, so a schema rejection or
    /// tool crash reaches the command output instead of a manual dig.
    #[test]
    fn claude_failure_surfaces_result_cause_and_transcript() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        write_eval_fixture(root)?;
        let _fake_bin_guard = fake_bin_lock();
        write_fake_bin(root, "git", fake_git_script())?;
        write_fake_bin(
            root,
            "claude",
            "#!/usr/bin/env bash\ncat >/dev/null\n\
             echo '{\"type\":\"system\",\"subtype\":\"init\"}'\n\
             echo '{\"type\":\"result\",\"subtype\":\"error_during_execution\",\"is_error\":true,\"result\":\"tool crashed: boom\"}'\n\
             exit 1\n",
        )?;
        let tools = fake_tools(root);

        let error = RunCommand {
            skill: DEFAULT_SKILL.to_string(),
            case_id: "case-a".to_string(),
            model: "fake-model".to_string(),
            repeats: 1,
            label: "crash".to_string(),
            skill_ref: None,
            skill_path: Some(root.join(DEFAULT_SKILL_PATH)),
            reviewer: None,
            backend: Backend::Claude,
            effort: None,
        }
        .run_with_tools(root, &tools)
        .unwrap_err();

        let message = format!("{error:#}");
        assert!(message.contains("error_during_execution"), "got: {message}");
        assert!(message.contains("tool crashed: boom"), "got: {message}");
        assert!(message.contains("transcript:"), "got: {message}");
        Ok(())
    }

    /// Effort is validated against the selected backend's own vocabulary,
    /// before any checkout or spend: `max` is a claude word codex does not
    /// know, and `minimal` is a codex word claude does not know. Each must be
    /// rejected on the wrong backend with no run directory created.
    #[test]
    fn effort_validation_is_backend_specific() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();

        let codex_error = RunCommand {
            skill: DEFAULT_SKILL.to_string(),
            case_id: "case-a".to_string(),
            model: "fake-model".to_string(),
            repeats: 1,
            label: "smoke".to_string(),
            skill_ref: None,
            skill_path: None,
            reviewer: None,
            backend: Backend::Codex,
            effort: Some("max".to_string()),
        }
        .run_with_tools(root, &fake_tools(root))
        .unwrap_err();
        assert!(codex_error.to_string().contains("unknown effort 'max'"));
        assert!(codex_error.to_string().contains("codex"));

        let claude_error = RunCommand {
            skill: DEFAULT_SKILL.to_string(),
            case_id: "case-a".to_string(),
            model: "fake-model".to_string(),
            repeats: 1,
            label: "smoke".to_string(),
            skill_ref: None,
            skill_path: None,
            reviewer: None,
            backend: Backend::Claude,
            effort: Some("minimal".to_string()),
        }
        .run_with_tools(root, &fake_tools(root))
        .unwrap_err();
        assert!(
            claude_error
                .to_string()
                .contains("unknown effort 'minimal'")
        );
        assert!(claude_error.to_string().contains("claude"));

        // Neither validation may create a run directory: both fail before spend.
        assert!(!root.join(RUN_ROOT).exists());
        Ok(())
    }

    /// Old run.json files predate the `backend` field. They must still parse,
    /// reading back as codex runs, so pre-backend artifacts keep comparing.
    #[test]
    fn run_json_without_backend_parses_as_codex() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("run.json");
        // A minimal run.json as written before the backend field existed.
        fs::write(
            &path,
            r#"{
              "id": "old-run",
              "skill": "pre-pr-review-swarm",
              "label": "old",
              "model": "gpt-5.4",
              "repeats": 1,
              "case_id": "case",
              "repo": "owner/repo",
              "subject_ref": "subject",
              "subject_sha": "subject",
              "base_ref": "base",
              "base_sha": "base",
              "skill_source": "working-tree",
              "skill_path": "agent-skills/pre-pr-review-swarm",
              "created_at": "2026-01-01T00:00:00Z"
            }"#,
        )?;
        let metadata: RunMetadata = read_json(&path)?;
        assert_eq!(metadata.backend, Backend::Codex);
        Ok(())
    }

    /// Cross-backend compare is the A/B axis this change exists to enable, but
    /// only when both runs pinned an explicit effort: codex "high" and claude
    /// "high" are different operating points, and an unset effort is each
    /// vendor's own default, so a default cannot be compared across backends.
    /// This pins both halves — allowed with efforts, refused without — and that
    /// the comparison records both backends. Same-backend effort mismatch stays
    /// covered by `compare_rejects_mismatched_cases_and_diffs`.
    #[test]
    fn compare_allows_cross_backend_only_with_explicit_efforts() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        write_eval_fixture(root)?;
        let _fake_bin_guard = fake_bin_lock();
        write_fake_bin(root, "git", fake_git_script())?;
        write_fake_bin(root, "codex", fake_codex_script())?;
        let tools = fake_tools(root);

        let baseline = root.join(RUN_ROOT).join("baseline-run");
        let candidate = root.join(RUN_ROOT).join("candidate-run");
        write_baseline_marker(&baseline)?;
        let mut baseline_meta = run_meta("baseline-run", "fake-model");
        baseline_meta.backend = Backend::Codex;
        baseline_meta.effort = Some("high".to_string());
        write_json(&baseline.join("run.json"), &baseline_meta)?;
        write_json(
            &baseline.join("repeat-1/findings.json"),
            &FindingsFile {
                findings: vec![finding("B1", "baseline bug", 1)],
            },
        )?;
        write_json(
            &candidate.join("repeat-1/findings.json"),
            &FindingsFile {
                findings: vec![finding("C1", "candidate bug", 1)],
            },
        )?;

        // Unset candidate effort: refused, and the message explains why.
        let mut unset = run_meta("candidate-run", "fake-model");
        unset.backend = Backend::Claude;
        unset.effort = None;
        write_json(&candidate.join("run.json"), &unset)?;
        let error = CompareCommand {
            baseline: baseline.strip_prefix(root)?.to_path_buf(),
            candidate: candidate.strip_prefix(root)?.to_path_buf(),
            model: Some("fake-model".to_string()),
            backend: Backend::Codex,
            effort: None,
        }
        .run_with_tools(root, &tools)
        .unwrap_err();
        assert!(error.to_string().contains("cross-backend"));
        assert!(error.to_string().contains("comparable across backends"));

        // Both efforts pinned: allowed, and comparison records both backends.
        let mut pinned = run_meta("candidate-run", "fake-model");
        pinned.backend = Backend::Claude;
        pinned.effort = Some("high".to_string());
        write_json(&candidate.join("run.json"), &pinned)?;
        CompareCommand {
            baseline: baseline.strip_prefix(root)?.to_path_buf(),
            candidate: candidate.strip_prefix(root)?.to_path_buf(),
            model: Some("fake-model".to_string()),
            backend: Backend::Codex,
            effort: None,
        }
        .run_with_tools(root, &tools)?;

        let comparison_path = fs::read_dir(root.join(RUN_ROOT))?
            .filter_map(Result::ok)
            .map(|entry| entry.path().join("comparison.json"))
            .find(|path| path.exists())
            .expect("expected comparison output");
        let comparison: ComparisonFile = read_json(&comparison_path)?;
        assert_eq!(comparison.baseline_backend, Backend::Codex);
        assert_eq!(comparison.candidate_backend, Backend::Claude);
        Ok(())
    }

    /// The full compare-then-synthesize pipeline must work on the claude
    /// backend, not just `run`: judges, matcher, and synthesis all flow through
    /// `run_agent`, and the claude `structured_output` extraction must produce
    /// judgments/matches/suggestions files the downstream parsers accept. Runs
    /// with `--model` omitted so the candidate-model default is exercised on
    /// its coherent path (judge backend == candidate backend). This is the
    /// claude analog of `compare_and_synthesize_use_fake_codex_artifacts`.
    #[test]
    fn compare_and_synthesize_use_fake_claude_artifacts() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        write_eval_fixture(root)?;
        let _fake_bin_guard = fake_bin_lock();
        write_fake_bin(root, "git", fake_git_script())?;
        write_fake_bin(root, "claude", fake_claude_script())?;
        let tools = fake_tools(root);

        let baseline = root.join(RUN_ROOT).join("baseline-run");
        let candidate = root.join(RUN_ROOT).join("candidate-run");
        let mut baseline_meta = run_meta("baseline-run", "fake-model");
        baseline_meta.backend = Backend::Claude;
        let mut candidate_meta = run_meta("candidate-run", "fake-model");
        candidate_meta.backend = Backend::Claude;
        write_json(&baseline.join("run.json"), &baseline_meta)?;
        write_json(&candidate.join("run.json"), &candidate_meta)?;
        write_baseline_marker(&baseline)?;
        write_json(
            &baseline.join("repeat-1/findings.json"),
            &FindingsFile {
                findings: vec![finding("B1", "baseline bug", 1)],
            },
        )?;
        write_json(
            &candidate.join("repeat-1/findings.json"),
            &FindingsFile {
                findings: vec![finding("C1", "candidate bug", 1)],
            },
        )?;

        CompareCommand {
            baseline: baseline.strip_prefix(root)?.to_path_buf(),
            candidate: candidate.strip_prefix(root)?.to_path_buf(),
            model: None,
            backend: Backend::Claude,
            effort: None,
        }
        .run_with_tools(root, &tools)?;

        let comparison_path = fs::read_dir(root.join(RUN_ROOT))?
            .filter_map(Result::ok)
            .map(|entry| entry.path().join("comparison.json"))
            .find(|path| path.exists())
            .expect("expected comparison output");
        let comparison: ComparisonFile = read_json(&comparison_path)?;
        assert_eq!(comparison.likely_regressions.len(), 1);
        assert_eq!(comparison.candidate_backend, Backend::Claude);

        SynthesizeCommand {
            comparison: comparison_path.strip_prefix(root)?.to_path_buf(),
            model: None,
            backend: Backend::Claude,
            effort: None,
        }
        .run_with_tools(root, &tools)?;
        let suggestions: SuggestionsFile =
            read_json(&comparison_path.parent().unwrap().join("suggestions.json"))?;
        assert_eq!(suggestions.suggestions.len(), 1);

        Ok(())
    }

    /// The judge model defaults to the candidate run's model, which is only
    /// coherent when the judge backend matches the backend that model belongs
    /// to. A claude candidate compared with the (default) codex judge backend
    /// and no --model must fail up front — before checkout or spend — rather
    /// than handing a claude model id to codex mid-run.
    #[test]
    fn compare_requires_explicit_model_when_judge_backend_differs() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        write_eval_fixture(root)?;

        let baseline = root.join(RUN_ROOT).join("baseline-run");
        let candidate = root.join(RUN_ROOT).join("candidate-run");
        let mut baseline_meta = run_meta("baseline-run", "claude-model");
        baseline_meta.backend = Backend::Claude;
        let mut candidate_meta = run_meta("candidate-run", "claude-model");
        candidate_meta.backend = Backend::Claude;
        write_json(&baseline.join("run.json"), &baseline_meta)?;
        write_json(&candidate.join("run.json"), &candidate_meta)?;
        write_baseline_marker(&baseline)?;

        let error = CompareCommand {
            baseline: baseline.strip_prefix(root)?.to_path_buf(),
            candidate: candidate.strip_prefix(root)?.to_path_buf(),
            model: None,
            backend: Backend::Codex,
            effort: None,
        }
        .run_with_tools(root, &fake_tools(root))
        .unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("cannot be the default judge model"),
            "got: {message}"
        );
        assert!(
            message.contains("pass --model explicitly"),
            "got: {message}"
        );
        Ok(())
    }

    /// Synthesize has the same defaulting rule as compare: the comparison's
    /// candidate model is only a usable default when the synthesis backend
    /// matches the recorded candidate backend.
    #[test]
    fn synthesize_requires_explicit_model_when_backend_differs() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        write_eval_fixture(root)?;
        let comparison_path = root.join(RUN_ROOT).join("comparison-run/comparison.json");
        write_json(
            &comparison_path,
            &ComparisonFile {
                baseline_run: "baseline-run".to_string(),
                candidate_run: "candidate-run".to_string(),
                case_id: "case".to_string(),
                candidate_model: Some("claude-model".to_string()),
                baseline_backend: Backend::Claude,
                candidate_backend: Backend::Claude,
                matches: Vec::new(),
                likely_regressions: Vec::new(),
                nondeterminism_notes: Vec::new(),
            },
        )?;

        let error = SynthesizeCommand {
            comparison: comparison_path.strip_prefix(root)?.to_path_buf(),
            model: None,
            backend: Backend::Codex,
            effort: None,
        }
        .run_with_tools(root, &fake_tools(root))
        .unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("cannot be the default synthesis model"),
            "got: {message}"
        );
        Ok(())
    }

    /// Old comparison.json files predate the backend fields. They must still
    /// parse — synthesize reads comparison.json from disk — reading both
    /// backends back as codex, mirroring `run_json_without_backend_parses_as_codex`.
    #[test]
    fn comparison_json_without_backends_parses_as_codex() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("comparison.json");
        // A minimal comparison.json as written before the backend fields existed.
        fs::write(
            &path,
            r#"{
              "baseline_run": "old-baseline",
              "candidate_run": "old-candidate",
              "case_id": "case",
              "candidate_model": "gpt-5.4",
              "matches": [],
              "likely_regressions": [],
              "nondeterminism_notes": []
            }"#,
        )?;
        let comparison: ComparisonFile = read_json(&path)?;
        assert_eq!(comparison.baseline_backend, Backend::Codex);
        assert_eq!(comparison.candidate_backend, Backend::Codex);
        Ok(())
    }

    /// A claude run that exits 0 while its terminal result event reports an
    /// error (the background-wait diagnostic shape, for example) must fail
    /// with the cause and transcript path — not be accepted as success. Before
    /// this check, the error diagnostic could be written to the last-message
    /// file and surface later as a confusing serde parse error.
    #[test]
    fn claude_zero_exit_error_result_fails_run() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        write_eval_fixture(root)?;
        let _fake_bin_guard = fake_bin_lock();
        write_fake_bin(root, "git", fake_git_script())?;
        write_fake_bin(
            root,
            "claude",
            "#!/usr/bin/env bash\ncat >/dev/null\n\
             echo '{\"type\":\"system\",\"subtype\":\"init\"}'\n\
             echo '{\"type\":\"result\",\"subtype\":\"error_during_execution\",\"is_error\":true,\"result\":\"Background tasks still running after 600s\"}'\n\
             exit 0\n",
        )?;

        let error = RunCommand {
            skill: DEFAULT_SKILL.to_string(),
            case_id: "case-a".to_string(),
            model: "fake-model".to_string(),
            repeats: 1,
            label: "zero-exit-error".to_string(),
            skill_ref: None,
            skill_path: Some(root.join(DEFAULT_SKILL_PATH)),
            reviewer: None,
            backend: Backend::Claude,
            effort: None,
        }
        .run_with_tools(root, &fake_tools(root))
        .unwrap_err();

        let message = format!("{error:#}");
        assert!(message.contains("reported an error"), "got: {message}");
        assert!(
            message.contains("Background tasks still running"),
            "got: {message}"
        );
        assert!(message.contains("transcript:"), "got: {message}");
        Ok(())
    }

    /// A zero-exit claude run whose success result carries no structured
    /// output has no parseable answer: with --json-schema enforced that shape
    /// means something went wrong, and there is deliberately no fallback to
    /// the prose result string (it would land in a .findings.json every
    /// consumer parses as schema JSON). The run must fail naming the
    /// transcript instead of writing a bogus last-message file.
    #[test]
    fn claude_zero_exit_without_structured_output_fails_run() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        write_eval_fixture(root)?;
        let _fake_bin_guard = fake_bin_lock();
        write_fake_bin(root, "git", fake_git_script())?;
        write_fake_bin(
            root,
            "claude",
            "#!/usr/bin/env bash\ncat >/dev/null\n\
             echo '{\"type\":\"system\",\"subtype\":\"init\"}'\n\
             echo '{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"result\":\"prose answer, not schema JSON\",\"usage\":{\"output_tokens\":5}}'\n\
             exit 0\n",
        )?;

        let error = RunCommand {
            skill: DEFAULT_SKILL.to_string(),
            case_id: "case-a".to_string(),
            model: "fake-model".to_string(),
            repeats: 1,
            label: "no-structured".to_string(),
            skill_ref: None,
            skill_path: Some(root.join(DEFAULT_SKILL_PATH)),
            reviewer: None,
            backend: Backend::Claude,
            effort: None,
        }
        .run_with_tools(root, &fake_tools(root))
        .unwrap_err();

        let message = format!("{error:#}");
        assert!(message.contains("no structured output"), "got: {message}");
        assert!(message.contains("transcript:"), "got: {message}");
        Ok(())
    }
}
