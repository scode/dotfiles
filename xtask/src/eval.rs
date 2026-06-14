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
}

#[derive(Debug, Args)]
struct SynthesizeCommand {
    #[arg(long)]
    comparison: PathBuf,
    #[arg(long)]
    model: Option<String>,
}

impl RunCommand {
    fn run_with_tools(self, root: &Path, tools: &ToolEnv) -> Result<()> {
        ensure!(
            self.skill == DEFAULT_SKILL,
            "only {DEFAULT_SKILL} is supported in v1"
        );
        ensure!(self.repeats > 0, "--repeats must be at least 1");

        let case = load_cases(root)?
            .remove(&self.case_id)
            .ok_or_else(|| anyhow!("unknown eval case '{}'", self.case_id))?;
        let target = prepare_case_checkout(root, &case, tools)?;
        let skill = resolve_skill(
            root,
            self.skill_ref.as_deref(),
            self.skill_path.as_deref(),
            tools,
        )?;
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
            skill_source: skill.source,
            skill_path: skill.path.display().to_string(),
            created_at: now_rfc3339()?,
        };
        write_json(&run_dir.join("run.json"), &metadata)?;

        let prompt_template = fs::read_to_string(root.join(EVAL_ROOT).join("prompts/run.md"))?;
        let schema = root.join(EVAL_ROOT).join("schemas/findings.schema.json");
        for repeat in 1..=self.repeats {
            let repeat_dir = run_dir.join(format!("repeat-{repeat}"));
            fs::create_dir_all(&repeat_dir)?;
            let prompt = prompt_template
                .replace("{{skill_path}}", &skill.path.display().to_string())
                .replace("{{repo_path}}", &target.checkout.display().to_string())
                .replace("{{base_sha}}", &target.base_sha)
                .replace("{{subject_sha}}", &target.subject_sha);
            let findings_path = repeat_dir.join("findings.json");
            let transcript_path = repeat_dir.join("transcript.jsonl");
            run_codex_json(
                tools,
                &self.model,
                &target.checkout,
                &schema,
                &findings_path,
                &transcript_path,
                &prompt,
            )?;
            let findings: FindingsFile = read_json(&findings_path)?;
            write_json(&findings_path, &findings)?;
        }

        println!("{}", run_dir.display());
        Ok(())
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
        let model = self.model.unwrap_or_else(|| candidate_meta.model.clone());
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
            },
            tools,
        )?;

        let judgments = judge_findings(
            root,
            tools,
            &model,
            &target,
            &comparison_dir,
            "baseline",
            &baseline_findings,
        )?;
        let candidate_judgments = judge_findings(
            root,
            tools,
            &model,
            &target,
            &comparison_dir,
            "candidate",
            &candidate_findings,
        )?;
        let matches = match_findings(
            root,
            tools,
            &model,
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
            &model,
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

#[derive(Debug, Clone, Deserialize, Serialize)]
struct EvalCase {
    id: String,
    repo: String,
    subject_ref: String,
    #[serde(default)]
    base_ref: Option<String>,
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

fn run_codex_json(
    tools: &ToolEnv,
    model: &str,
    cwd: &Path,
    schema: &Path,
    output_last_message: &Path,
    transcript: &Path,
    prompt: &str,
) -> Result<()> {
    let mut child = Command::new(&tools.codex)
        .arg("exec")
        .arg("--json")
        .arg("--ephemeral")
        .arg("--ignore-user-config")
        .arg("--ignore-rules")
        .arg("-s")
        .arg("read-only")
        .arg("-m")
        .arg(model)
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
    model: &str,
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
        model,
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
    model: &str,
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
        model,
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
        }
        .run_with_tools(root, &tools)?;

        let run_root = root.join(RUN_ROOT);
        let run_dir = fs::read_dir(&run_root)?
            .next()
            .expect("expected run dir")?
            .path();
        assert!(run_dir.join("run.json").is_file());
        assert!(run_dir.join("repeat-1/findings.json").is_file());
        assert!(run_dir.join("repeat-2/transcript.jsonl").is_file());
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

        for schema in [
            "findings.schema.json",
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
        }
        .run_with_tools(root, &tools)
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("must review the same resolved diff")
        );
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
            repeat: Some(repeat),
        }
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
"#,
        )?;
        fs::write(
            root.join(EVAL_ROOT).join("prompts/run.md"),
            "run {{skill_path}} {{repo_path}} {{base_sha}}..{{subject_sha}}",
        )?;
        fs::write(root.join(EVAL_ROOT).join("prompts/judge.md"), "judge")?;
        fs::write(root.join(EVAL_ROOT).join("prompts/match.md"), "match")?;
        fs::write(
            root.join(EVAL_ROOT).join("prompts/synthesize.md"),
            "synthesize",
        )?;
        for schema in [
            "findings.schema.json",
            "judgments.schema.json",
            "matches.schema.json",
            "suggestions.schema.json",
        ] {
            fs::write(root.join(EVAL_ROOT).join("schemas").join(schema), "{}")?;
        }
        fs::write(root.join(DEFAULT_SKILL_PATH).join("SKILL.md"), "# skill")?;
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
  fetch) exit 0 ;;
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
saw_ephemeral=0
saw_read_only=0
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
  elif [ "$1" = "--ephemeral" ]; then
    saw_ephemeral=1
    shift
  elif [ "$1" = "--ignore-user-config" ]; then
    saw_ignore_user_config=1
    shift
  elif [ "$1" = "--ignore-rules" ]; then
    saw_ignore_rules=1
    shift
  elif [ "$1" = "-s" ]; then
    test "$2" = "read-only"
    saw_read_only=1
    shift 2
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
test "$saw_read_only" = "1"
test "$saw_ignore_user_config" = "1"
test "$saw_ignore_rules" = "1"
test -n "$cwd"
test -n "$stdin_prompt"
mkdir -p "$(dirname "$out")"
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
