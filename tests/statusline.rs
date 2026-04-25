use std::process::Command;

fn run_statusline(input: serde_json::Value, path: Option<String>) -> Vec<u8> {
    let mut command = Command::new("sh");
    command
        .arg("payload/bin/claude-statusline.sh")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped());
    if let Some(path) = path {
        command.env("PATH", path);
    }

    let output = command
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .as_mut()
                .unwrap()
                .write_all(input.to_string().as_bytes())?;
            child.wait_with_output()
        })
        .unwrap();

    assert!(
        output.status.success(),
        "statusline failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

#[test]
fn statusline_strips_control_characters_from_untrusted_text() {
    let cwd = tempfile::tempdir().unwrap();
    let input = serde_json::json!({
        "workspace": {
            "current_dir": cwd.path().join("bad\u{1b}dir").to_string_lossy()
        },
        "model": {
            "display_name": "model\u{1b}]0;bad\u{7}"
        },
        "context_window": {
            "used_percentage": 0
        }
    });

    let stdout = String::from_utf8(run_statusline(input, None)).unwrap();
    assert!(
        stdout.contains("baddir"),
        "statusline should keep printable directory text: {stdout:?}"
    );
    assert!(
        stdout.contains("model]0;bad"),
        "statusline should keep printable model text: {stdout:?}"
    );
    assert!(
        !stdout.contains("\u{1b}]0;bad"),
        "statusline should strip control sequences from model text: {stdout:?}"
    );
    assert!(
        !stdout.contains("bad\u{1b}dir") && !stdout.contains('\u{7}'),
        "statusline should strip untrusted control bytes: {stdout:?}"
    );
}

#[test]
fn statusline_strips_control_characters_from_git_branch() {
    let cwd = tempfile::tempdir().unwrap();
    let bin = tempfile::tempdir().unwrap();
    let fake_git = bin.path().join("git");
    std::fs::write(
        &fake_git,
        r#"#!/bin/sh
if [ "$3" = rev-parse ]; then
  exit 0
fi
if [ "$3" = branch ]; then
  printf 'branch\233bad\n'
  exit 0
fi
exit 0
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&fake_git).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o755);
    std::fs::set_permissions(&fake_git, permissions).unwrap();

    let path = format!(
        "{}:{}",
        bin.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let input = serde_json::json!({
        "workspace": {
            "current_dir": cwd.path()
        },
        "model": {
            "display_name": "model"
        },
        "context_window": {
            "used_percentage": 0
        }
    });

    let stdout = String::from_utf8(run_statusline(input, Some(path))).unwrap();
    assert!(
        stdout.contains("git:(branchbad)"),
        "statusline should keep printable branch text: {stdout:?}"
    );
    assert!(
        !stdout.as_bytes().contains(&0x9b),
        "statusline should strip raw C1 control bytes: {stdout:?}"
    );
}
