use std::{
    path::Path,
    process::{Command, Stdio},
};

fn format(dirty: bool, commit_id: &str, branches: &str) -> (String, Option<String>) {
    // no branches or multiple
    if branches.is_empty() || branches.contains(' ') {
        return (commit_id.to_owned(), None);
    }
    let info = format!(
        "{}@{commit_id:.7}{}",
        branches.strip_suffix('*').unwrap_or(branches),
        if dirty { "*" } else { "" },
    );
    (commit_id.to_owned(), Some(info))
}

// why not just &[&str]?
// that, detective, is the right question
fn sh<'a>(args: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let mut args = args.into_iter();
    Command::new(args.next()?)
        .args(args)
        .stdout(Stdio::piped())
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|mut s| {
            s.truncate(s.trim_end().len());
            s
        })
}

fn get_from_jj() -> Option<(String, Option<String>)> {
    // avoid jj snapshots when RA calls this
    if std::env::var("RA_RUSTC_WRAPPER").is_ok() {
        return Some(("rust-analyzer".into(), None));
    }
    // ..or clippy
    if std::env::var("CARGO_CFG_CLIPPY").is_ok() {
        return Some(("clippy".into(), None));
    }

    // just because I'm turbo conscious about this fabulous build script
    // making a quadrillion snapshots cuz it's bugged or something
    //
    // but basically every invocation of `cargo run` leaves an exact snapshot
    // of the code it runs in the jj oplog, which is occasionally useful
    sh([
        "notify-send",
        "--app-name=Noita", // this does use the icon from steam lul
        "JJ snapshot",
        "Made a jj snapshot of a project you're working on",
    ]);

    let res = sh([
        "jj",
        "log",
        "--no-graph",
        "-r",
        "@|@-",
        "-T",
        r#"concat(empty,",",commit_id,",",bookmarks,"\n")"#,
    ])?;

    let (wc, parent) = res.split_once('\n')?;

    let (is_empty, rest) = wc.split_once(',')?;
    let (commit_id, branches) = rest.split_once(',')?;
    if is_empty != "true" {
        return Some(format(true, commit_id, branches));
    }

    let (commit_id, branches) = parent.split_once(',')?.1.split_once(',')?;
    Some(format(false, commit_id, branches))
}

// we need to run *3* git commands to get all the information
fn get_from_git() -> Option<(String, Option<String>)> {
    let is_empty = sh(["git", "diff", "--shortstat"])?.is_empty();
    let commit_id = sh(["git", "rev-parse", "HEAD"])?;
    let branch = sh(["git", "name-rev", "--name-only", "--refs=heads/*", "HEAD"])?;

    let branch = branch.strip_suffix("~1").unwrap_or_default();

    Some(format(!is_empty, &commit_id, branch))
}

fn main() {
    // either git or colocated
    if Path::new(".git/HEAD").exists() {
        println!("cargo::rerun-if-changed=.git/HEAD");
    } else if Path::new(".jj/repo/op_heads/heads").exists() {
        // maybe *someone* will clone this with jj without colocation lol
        println!("cargo::rerun-if-changed=.jj/repo/op_heads/heads");
    }
    println!("cargo::rerun-if-env-changed=JJ_COMMIT");

    let (commit, info) = get_from_jj()
        .or_else(get_from_git)
        .expect("Building without jj or git installed, or not in a repo");

    println!("cargo::rustc-env=JJ_COMMIT={commit}");
    if let Some(info) = info {
        println!("cargo::rustc-env=JJ_INFO={info}");
    }
}
