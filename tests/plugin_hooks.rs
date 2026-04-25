use std::process::Command;

fn run_hook(script: &str, env_value: &str) -> serde_json::Value {
    let output = Command::new("bash")
        .arg(script)
        .env("CLAUDE_PLUGIN_ROOT", env_value)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "hook failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn session_hooks_escape_plugin_root_in_json() {
    for (script, expected) in [
        (
            "claude-code-marketplace/codex-code-review/hooks/session-start.sh",
            "CODEX_REVIEW_BIN=/tmp/plugin \"quoted\"\npath/bin",
        ),
        (
            "claude-code-marketplace/gemini-code-review/hooks/session-start.sh",
            "GEMINI_REVIEW_BIN=/tmp/plugin \"quoted\"\npath/bin",
        ),
    ] {
        let json = run_hook(script, "/tmp/plugin \"quoted\"\npath");

        assert_eq!(json["hookSpecificOutput"]["additionalContext"], expected);
    }
}
