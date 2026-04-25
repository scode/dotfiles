use std::process::Command;

fn run_pdfnopassword(input_name: &str, password: &str) -> Option<(String, String)> {
    let work = tempfile::tempdir().unwrap();
    let bin = work.path().join("bin");
    std::fs::create_dir(&bin).unwrap();
    let args_file = work.path().join("pdftk-args");
    let fake_pdftk = bin.join("pdftk");
    std::fs::write(
        &fake_pdftk,
        r#"#!/bin/sh
printf '%s\n' "$@" > "$PDFTK_ARGS_FILE"
input=$1
shift
while [ "$#" -gt 0 ]; do
  if [ "$1" = output ]; then
    cp "$input" "$2"
    exit 0
  fi
  shift
done
exit 1
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&fake_pdftk).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o755);
    std::fs::set_permissions(&fake_pdftk, permissions).unwrap();

    std::fs::write(work.path().join(input_name), "pdf").unwrap();

    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let output = match Command::new("zsh")
        .arg("-fc")
        .arg(r#"source "$1"; cd "$2"; pdfnopassword "$3" "$4""#)
        .arg("zsh")
        .arg(format!(
            "{}/old/dotfiles/zcommon",
            env!("CARGO_MANIFEST_DIR")
        ))
        .arg(work.path())
        .arg(input_name)
        .arg(password)
        .env("HOME", work.path())
        .env("PATH", path)
        .env("PDFTK_ARGS_FILE", &args_file)
        .output()
    {
        Ok(output) => output,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("skipping legacy zcommon smoke test because zsh is not installed");
            return None;
        }
        Err(e) => panic!("failed to run zsh: {e}"),
    };

    assert!(
        output.status.success(),
        "pdfnopassword failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(work.path().join(input_name)).unwrap(),
        "pdf"
    );
    Some((
        String::from_utf8_lossy(&output.stderr).into_owned(),
        std::fs::read_to_string(args_file).unwrap(),
    ))
}

#[test]
fn pdfnopassword_quotes_filename_and_keeps_password_out_of_trace() {
    let password = "pa ss;word";
    let Some((stderr, pdftk_args)) = run_pdfnopassword("file with spaces.pdf", password) else {
        return;
    };

    assert!(
        !stderr.contains(password),
        "password should not appear in shell trace"
    );
    assert_eq!(
        pdftk_args,
        "file with spaces.pdf.bak\ninput_pw\npa ss;word\noutput\nfile with spaces.pdf\n"
    );
}

#[test]
fn pdfnopassword_handles_leading_dash_filenames() {
    let Some((_, pdftk_args)) = run_pdfnopassword("-leading.pdf", "password") else {
        return;
    };

    assert_eq!(
        pdftk_args,
        "./-leading.pdf.bak\ninput_pw\npassword\noutput\n./-leading.pdf\n"
    );
}
