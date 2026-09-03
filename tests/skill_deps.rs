//! Enforces the cross-skill dependency contract for the layered agent skills.
//!
//! Some skills under `agent-skills/` depend on other skills (galaxy-brain on
//! the shell-out and routing skills, for example). There is no harness-level
//! way to declare that, so the contract is carried in text: a layered skill's
//! `SPEC.md` opens with a `Dependencies:` line, and its `SKILL.md` carries one
//! marked stanza per dependency telling an agent how to load it by name
//! through the harness's own skill mechanism and to stop if that fails. The
//! stanza is the same text in every skill, parameterized only by the
//! dependency's name, because there is no way to depend on a skill in order to
//! learn how to depend on skills.
//!
//! This test keeps the declaration and the stanzas in step: every declared
//! dependency has exactly one stanza, every stanza names a declared
//! dependency, stanzas live only in `SKILL.md`, and each stanza's wording
//! matches [`STANZA`] once whitespace is normalized (dprint re-wraps the
//! blockquote at 120 columns, and the dependency names differ in length, so a
//! byte comparison would never hold). Skills without a `Dependencies:` line
//! are outside the contract and must carry no stanza at all.
//!
//! The Codex branch of the stanza reads the dependency from
//! `${CODEX_HOME:-$HOME/.codex}/skills/<name>/SKILL.md`, because Codex has no
//! mid-turn skill loader and that is the skills root the Codex 0.152 binary
//! uses. Codex's public docs already describe a `.agents/skills` root; when
//! Codex changes its root, re-verify the path with a live `codex exec` run in
//! an isolated `CODEX_HOME`, then change [`STANZA`] here and every stanza in
//! the skills together.

use std::fs;
use std::path::{Path, PathBuf};

/// The canonical dependency stanza, with `{name}` standing for the dependency
/// skill's name. Skill files carry it as a blockquote between
/// `<!-- dependency: {name} -->` and `<!-- /dependency -->`; the comparison
/// strips the blockquote markers and collapses whitespace, so line breaks are
/// free to differ.
const STANZA: &str = "Load the skill `{name}` through your harness's skill mechanism: the Skill tool on Claude Code, \
the `skill` tool on OpenCode, the `read_skill` tool on Muse Code. On Codex, which has no such tool, read \
`${CODEX_HOME:-$HOME/.codex}/skills/{name}/SKILL.md`; if it is absent or unreadable, report that exact path and do \
not search elsewhere. On any other harness, use its skill loader only if the result reports the skill's base \
directory; otherwise stop and say this skill has not been verified on that harness. The base directory is the \
directory containing the loaded `SKILL.md`. Confirm the name the loader reports is `{name}`; if the loader shows no \
name, read only the frontmatter (the first lines up to the closing `---`) of `<base>/SKILL.md`. Read its sidecars \
relative to the base directory. Stop and tell the user that `{name}` is not installed or could not be loaded, naming \
the path or tool, if the loader reports the skill as unknown or denied, the file is absent or unreadable on Codex \
(the skills root for Codex 0.152), the result says it was truncated, the name does not match, or a sidecar this step \
needs is not readable under the base directory. Do not continue from memory, from a copy, from a search for the file \
elsewhere, or from a similar skill.";

/// The skills that are part of the layered contract. Each must carry the
/// `Dependencies:` line; without this list, deleting the line and the stanza
/// together would silently opt a skill out of the test. Add a skill here in
/// the PR that gives it a `Dependencies:` line.
const LAYERED_SKILLS: &[&str] = &["scode-galaxy-brain", "scode-harness-shellout"];

const OPEN_MARKER: &str = "<!-- dependency:";
const CLOSE_MARKER: &str = "<!-- /dependency -->";

fn skills_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("agent-skills")
}

/// Collapses every run of whitespace to one space so wrapped and unwrapped
/// forms of the same prose compare equal.
fn normalize(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The dependency names declared by a `SPEC.md`, or `None` when the file has
/// no `Dependencies:` line as its first non-heading, non-blank line (which
/// means the skill is not part of the layered contract). `Dependencies: none`
/// yields an empty list.
fn declared_dependencies(spec: &str) -> Option<Vec<String>> {
    let first = spec
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))?;
    let rest = first.strip_prefix("Dependencies:")?.trim();
    if rest == "none" {
        return Some(Vec::new());
    }
    Some(rest.split(',').map(|name| name.trim().to_owned()).collect())
}

/// Every marked stanza in a markdown file as `(dependency name, body)`, with
/// the body's blockquote markers stripped. The body must be a blockquote:
/// every non-blank line between the markers starts with `>`, so that a stanza
/// whose quoting was lost (and which no longer reads as a delimited block to
/// an agent) fails rather than passing on prose alone. Panics on an
/// unterminated block, a closing marker with no opening one, or a malformed
/// opening marker, since each means the file is broken rather than merely
/// non-compliant.
fn stanzas(path: &Path, text: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        assert!(
            trimmed != CLOSE_MARKER,
            "{}: closing dependency marker without an opening one",
            path.display()
        );
        let Some(rest) = trimmed.strip_prefix(OPEN_MARKER) else {
            continue;
        };
        let name = rest
            .strip_suffix("-->")
            .unwrap_or_else(|| panic!("{}: malformed marker {trimmed:?}", path.display()))
            .trim()
            .to_owned();
        let mut body = Vec::new();
        let mut closed = false;
        for inner in lines.by_ref() {
            if inner.trim() == CLOSE_MARKER {
                closed = true;
                break;
            }
            let stripped = inner.trim_start();
            if stripped.is_empty() {
                continue;
            }
            let quoted = stripped.strip_prefix('>').unwrap_or_else(|| {
                panic!(
                    "{}: stanza for {name} has a line outside the blockquote: {inner:?}",
                    path.display()
                )
            });
            body.push(quoted.trim());
        }
        assert!(
            closed,
            "{}: stanza for {name} has no closing marker",
            path.display()
        );
        found.push((name, body.join(" ")));
    }
    found
}

/// Every `.md` file under a skill directory, `lore/` included: the markers are
/// reserved throughout a skill, and a historical note that wants to quote a
/// stanza can do so without the markers.
fn markdown_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(markdown_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "md") {
            out.push(path);
        }
    }
    out
}

/// Why: a skill whose `SPEC.md` declares a dependency but whose `SKILL.md`
/// lacks the stanza (or carries a drifted copy) silently loses the fail-closed
/// loading behavior the whole layering rests on, and nothing at runtime would
/// notice. What: for every skill directory, the declared dependency set and
/// the set of marked stanzas in `SKILL.md` are equal, no other file carries a
/// stanza, and each stanza's normalized text equals [`STANZA`] with the name
/// substituted. Skills that declare nothing carry nothing.
#[test]
fn dependency_stanzas_match_declarations_and_template() {
    let mut layered_skills = 0;
    for entry in fs::read_dir(skills_dir()).unwrap() {
        let skill_dir = entry.unwrap().path();
        if !skill_dir.is_dir() {
            continue;
        }
        let skill = skill_dir
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let declared = fs::read_to_string(skill_dir.join("SPEC.md"))
            .ok()
            .and_then(|spec| declared_dependencies(&spec));

        let mut in_skill_md = Vec::new();
        for path in markdown_files(&skill_dir) {
            let text = fs::read_to_string(&path).unwrap();
            let found = stanzas(&path, &text);
            if path.file_name().is_some_and(|name| name == "SKILL.md")
                && path.parent() == Some(skill_dir.as_path())
            {
                in_skill_md.extend(found);
            } else {
                assert!(
                    found.is_empty(),
                    "{skill}: {} carries a dependency stanza; stanzas belong in SKILL.md only",
                    path.display()
                );
            }
        }

        let Some(declared) = declared else {
            assert!(
                !LAYERED_SKILLS.contains(&skill.as_str()),
                "{skill}: is a layered skill but its SPEC.md has no `Dependencies:` first line"
            );
            assert!(
                in_skill_md.is_empty(),
                "{skill}: SKILL.md carries dependency stanzas but SPEC.md declares no `Dependencies:` line"
            );
            continue;
        };
        layered_skills += 1;

        let mut declared_sorted = declared.clone();
        declared_sorted.sort();
        assert!(
            declared_sorted.iter().all(|name| !name.is_empty()),
            "{skill}: SPEC.md declares an empty dependency name"
        );
        declared_sorted.dedup();
        assert_eq!(
            declared_sorted.len(),
            declared.len(),
            "{skill}: SPEC.md declares a dependency twice"
        );
        let mut found_names: Vec<String> =
            in_skill_md.iter().map(|(name, _)| name.clone()).collect();
        found_names.sort();
        assert_eq!(
            found_names, declared_sorted,
            "{skill}: SPEC.md declares {declared_sorted:?} but SKILL.md carries stanzas for {found_names:?}"
        );

        for (name, body) in &in_skill_md {
            let expected = normalize(&STANZA.replace("{name}", name));
            assert_eq!(
                normalize(body),
                expected,
                "{skill}: the stanza for {name} does not match the canonical template"
            );
        }
    }
    assert_eq!(
        layered_skills,
        LAYERED_SKILLS.len(),
        "every skill in LAYERED_SKILLS should have been checked"
    );
}

/// Why: the template itself is prose that people will edit, and a stray
/// `{name}` left unsubstituted or a placeholder typo would pass the stanza
/// comparison in every skill while telling agents to load a skill literally
/// named `{name}`. What: substitution leaves no placeholder behind and the
/// template names the Codex root the doc comment above promises.
#[test]
fn stanza_template_is_well_formed() {
    let filled = STANZA.replace("{name}", "scode-example");
    assert!(!filled.contains("{name}"));
    assert!(filled.contains("${CODEX_HOME:-$HOME/.codex}/skills/scode-example/SKILL.md"));
    assert!(filled.contains("Confirm the name the loader reports is `scode-example`"));
}
