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
    /// Run a skill eval case through Codex.
    Run(RunCommand),
    /// Mark a completed run as a baseline.
    Baseline(BaselineCommand),
    /// Compare a candidate run against a baseline run.
    Compare(CompareCommand),
    /// Ask Codex to suggest skill changes for likely regressions.
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
    /// Reasoning effort for the run's agents (minimal|low|medium|high).
    ///
    /// Passed to codex as `model_reasoning_effort`. When omitted the run uses
    /// codex's built-in default — the harness ignores user config, so this
    /// flag is the only way to control effort. Effort changes what a run
    /// measures, so it is recorded in run.json and `compare` refuses to mix
    /// runs with different efforts.
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
    /// Reasoning effort for the judge and matcher agents
    /// (minimal|low|medium|high). Independent of the effort the compared runs
    /// used; when omitted the judges run at codex's built-in default.
    #[arg(long)]
    effort: Option<String>,
}

#[derive(Debug, Args)]
struct SynthesizeCommand {
    #[arg(long)]
    comparison: PathBuf,
    #[arg(long)]
    model: Option<String>,
    /// Reasoning effort for the synthesis agent (minimal|low|medium|high).
    /// When omitted, codex's built-in default.
    #[arg(long)]
    effort: Option<String>,
}

/// Reasoning-effort levels codex accepts for `model_reasoning_effort`.
/// Validated up front, like the reviewer restriction, so a typo fails before
/// any tokens are spent rather than mid-run inside codex.
const EFFORT_LEVELS: [&str; 4] = ["minimal", "low", "medium", "high"];

fn validate_effort(effort: Option<&str>) -> Result<()> {
    if let Some(effort) = effort {
        ensure!(
            EFFORT_LEVELS.contains(&effort),
            "unknown effort '{effort}'; expected one of {}",
            EFFORT_LEVELS.join(", ")
        );
    }
    Ok(())
}

impl RunCommand {
    fn run_with_tools(self, root: &Path, tools: &ToolEnv) -> Result<()> {
        ensure!(
            self.skill == DEFAULT_SKILL,
            "only {DEFAULT_SKILL} is supported in v1"
        );
        ensure!(self.repeats > 0, "--repeats must be at least 1");
        validate_effort(self.effort.as_deref())?;

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
            effort: self.effort.clone(),
            skill_source: skill.source,
            skill_path: skill.path.display().to_string(),
            created_at: now_rfc3339()?,
        };
        write_json(&run_dir.join("run.json"), &metadata)?;

        // The harness owns the fan-out: one codex agent per reviewer charter,
        // merged deterministically below. An earlier design asked a single
        // codex session to run the whole skill, trusting it to spawn reviewer
        // subagents; observed transcripts showed it never actually spawned
        // anything (collab waits on zero threads) and silently reviewed solo
        // with every charter loaded — the exact degraded mode the swarm
        // exists to avoid, with no way to detect it from the findings alone.
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

        let prompt_template = fs::read_to_string(root.join(EVAL_ROOT).join("prompts/reviewer.md"))?;
        let schema = root
            .join(EVAL_ROOT)
            .join("schemas/reviewer-findings.schema.json");
        let charters_dir = skill.path.join("reviewers");
        let context = PanelContext {
            tools,
            spec: ModelSpec {
                model: &self.model,
                effort: self.effort.as_deref(),
            },
            prompt_template: &prompt_template,
            schema: &schema,
            scope_path: &scope_path,
            charters_dir: &charters_dir,
            target: &target,
        };
        for repeat in 1..=self.repeats {
            let repeat_dir = run_dir.join(format!("repeat-{repeat}"));
            let findings = run_reviewer_panel(&context, &panel, &repeat_dir)?;
            write_json(&repeat_dir.join("findings.json"), &findings)?;
        }

        println!("{}", run_dir.display());
        Ok(())
    }
}

/// How many reviewer agents run concurrently within one repeat. The agents
/// are network-bound codex processes, so modest parallelism cuts wall-clock
/// substantially without saturating the machine or the API.
const REVIEWER_CONCURRENCY: usize = 6;

/// Select the spawnable reviewer charters from a resolved skill.
///
/// The charter files themselves carry the exclusion markers, so this works
/// across skill versions without a separate manifest: shared base charters
/// (read by lens reviewers, never spawned) say "not spawned as a reviewer",
/// and condition-gated charters (spec-compliance) say "Only spawned when
/// `SPEC.md` exists". Text markers are fragile in principle, but they are
/// versioned with the skill export being evaluated, which a harness-side
/// manifest would not be.
fn discover_panel(skill_path: &Path, target_root: &Path) -> Result<Vec<String>> {
    let dir = skill_path.join("reviewers");
    let mut panel = Vec::new();
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
        let text = fs::read_to_string(&path)?;
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

/// Per-repeat ground-truth record of the fan-out: which reviewer agents ran
/// and how many findings each returned. A repeat's findings.json only reaches
/// disk after every panel member completed — any reviewer failure aborts the
/// run — so this file is the evidence that the swarm actually executed,
/// replacing an agent's unverifiable self-report.
#[derive(Debug, Serialize, Deserialize)]
struct ExecutionRecord {
    expected: usize,
    reviewers: Vec<ReviewerExecution>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReviewerExecution {
    reviewer: String,
    findings: usize,
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

/// Run every panel charter as its own codex agent (bounded concurrency),
/// then merge: namespace finding ids by reviewer and stamp harness-side
/// attribution. Attribution stamped here is ground truth — it cannot be
/// fabricated or dropped by a coordinator agent.
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
    run_codex_json(
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
        ensure!(
            baseline_meta.effort == candidate_meta.effort,
            "baseline effort ({}) does not match candidate ({})",
            baseline_meta.effort.as_deref().unwrap_or("codex default"),
            candidate_meta.effort.as_deref().unwrap_or("codex default")
        );
        validate_effort(self.effort.as_deref())?;
        let model = self.model.unwrap_or_else(|| candidate_meta.model.clone());
        let spec = ModelSpec {
            model: &model,
            effort: self.effort.as_deref(),
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
        let model = self
            .model
            .or_else(|| comparison.candidate_model.clone())
            .ok_or_else(|| anyhow!("--model is required when comparison has no candidate model"))?;
        validate_effort(self.effort.as_deref())?;
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
        run_codex_json(
            tools,
            ModelSpec {
                model: &model,
                effort: self.effort.as_deref(),
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
}

impl Default for ToolEnv {
    fn default() -> Self {
        Self {
            git: PathBuf::from("git"),
            codex: PathBuf::from("codex"),
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

/// Model id plus optional reasoning effort — the two knobs every codex
/// invocation takes together. Bundled so a call site cannot choose one
/// without deciding the other.
#[derive(Clone, Copy)]
struct ModelSpec<'a> {
    model: &'a str,
    effort: Option<&'a str>,
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
    child
        .stdin
        .as_mut()
        .ok_or_else(|| anyhow!("codex stdin unavailable"))?
        .write_all(prompt.as_bytes())?;
    let output = child
        .wait_with_output()
        .context("failed to run codex exec")?;
    fs::write(transcript, &output.stdout)?;
    if !output.status.success() {
        bail!(
            "codex exec failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    ensure!(
        output_last_message.is_file(),
        "codex did not write {}",
        output_last_message.display()
    );
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
    run_codex_json(
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
    run_codex_json(
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
        // Per-reviewer artifacts: the fixture panel is docs-comments and
        // test-quality (correctness is a base charter, spec-compliance is
        // gated on a SPEC.md the fake checkout lacks).
        for repeat in ["repeat-1", "repeat-2"] {
            for reviewer in ["docs-comments", "test-quality"] {
                assert!(
                    run_dir
                        .join(repeat)
                        .join("reviewers")
                        .join(format!("{reviewer}.findings.json"))
                        .is_file()
                );
                assert!(
                    run_dir
                        .join(repeat)
                        .join("reviewers")
                        .join(format!("{reviewer}.transcript.jsonl"))
                        .is_file()
                );
            }
        }
        let execution: ExecutionRecord = read_json(&run_dir.join("repeat-1/execution.json"))?;
        assert_eq!(execution.expected, 2);
        assert_eq!(execution.reviewers.len(), 2);
        // The merged findings carry harness-stamped attribution and
        // reviewer-namespaced ids — ground truth, not agent self-report.
        let findings: FindingsFile = read_json(&run_dir.join("repeat-1/findings.json"))?;
        assert_eq!(findings.findings.len(), 2);
        let ids: Vec<_> = findings.findings.iter().map(|f| f.id.as_str()).collect();
        assert!(ids.contains(&"docs-comments:F1"));
        assert!(ids.contains(&"test-quality:F1"));
        for finding in &findings.findings {
            let stamped = finding.reviewers.as_ref().expect("stamped attribution");
            assert_eq!(stamped.len(), 1);
            assert!(finding.id.starts_with(&format!("{}:", stamped[0])));
        }
        let metadata: RunMetadata = read_json(&run_dir.join("run.json"))?;
        assert_eq!(metadata.base_sha, "base-sha");
        assert_eq!(metadata.subject_sha, "subject-remote-sha");

        BaselineCommand {
            run: run_dir.strip_prefix(root)?.to_path_buf(),
        }
        .run(root)?;
        assert!(run_dir.join("baseline.json").is_file());

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

        fs::write(target_root.join("SPEC.md"), "# spec")?;
        let panel = discover_panel(&skill_path, &target_root)?;
        assert_eq!(
            panel,
            vec!["docs-comments", "spec-compliance", "test-quality"]
        );
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
        write_fake_bin(root, "git", fake_git_script())?;
        write_fake_bin(
            root,
            "codex",
            "#!/usr/bin/env bash\ncat >/dev/null\necho 'fake reviewer crash' >&2\nexit 1\n",
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
            effort: None,
        }
        .run_with_tools(root, &tools)
        .unwrap_err();

        let message = format!("{error:#}");
        assert!(message.contains("reviewer '"));
        assert!(message.contains("failed"));
        Ok(())
    }

    /// A typo'd reviewer name must fail before any tokens are spent, not
    /// silently run the full panel with a no-op restriction note.
    #[test]
    fn run_command_rejects_unknown_reviewer() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        write_eval_fixture(root)?;
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
            effort: Some("high".to_string()),
        }
        .run_with_tools(root, &tools)?;

        let run_dir = fs::read_dir(root.join(RUN_ROOT))?
            .next()
            .expect("expected run dir")?
            .path();
        let metadata: RunMetadata = read_json(&run_dir.join("run.json"))?;
        assert_eq!(metadata.effort.as_deref(), Some("high"));
        let config = fs::read_to_string(
            run_dir.join("repeat-1/reviewers/test-quality.findings.json.config"),
        )?;
        assert!(config.contains("model_reasoning_effort=high"));
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
                matches: Vec::new(),
                likely_regressions: Vec::new(),
                nondeterminism_notes: Vec::new(),
            },
        )?;

        let error = SynthesizeCommand {
            comparison: comparison_path.strip_prefix(root)?.to_path_buf(),
            model: None,
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
            effort: None,
        }
        .run_with_tools(root, &tools)
        .unwrap_err();
        assert!(error.to_string().contains("reviewer restriction"));

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
            effort: None,
        }
        .run_with_tools(root, &tools)
        .unwrap_err();
        assert!(error.to_string().contains("baseline effort"));
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
        fs::write(root.join(EVAL_ROOT).join("prompts/judge.md"), "judge")?;
        fs::write(root.join(EVAL_ROOT).join("prompts/match.md"), "match")?;
        fs::write(
            root.join(EVAL_ROOT).join("prompts/synthesize.md"),
            "synthesize",
        )?;
        for schema in [
            "findings.schema.json",
            "reviewer-findings.schema.json",
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
echo '{"event":"done"}'
"#
    }
}
