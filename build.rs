use std::{
    env,
    path::Path,
    process::{Command, Stdio},
};

use winresource::WindowsResource;

fn format(dirty: bool, commit_id: &str, branches: &str) -> (String, String) {
    // no branches or multiple
    if branches.is_empty() || branches.contains(' ') {
        let info = format!("{commit_id:.7}{}", if dirty { "*" } else { "" });
        return (commit_id.to_owned(), info);
    }
    let info = format!(
        "{}@{commit_id:.7}{}",
        branches.strip_suffix('*').unwrap_or(branches),
        if dirty { "*" } else { "" },
    );
    (commit_id.to_owned(), info)
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

fn get_from_jj() -> Option<(String, String)> {
    // avoid jj snapshots when RA calls this
    if std::env::var("RA_RUSTC_WRAPPER").is_ok() {
        let stub = "rust-analyzer".to_owned();
        return Some((stub.clone(), stub));
    }
    // ..or clippy
    if std::env::var("CARGO_CFG_CLIPPY").is_ok() {
        let stub = "clippy".to_owned();
        return Some((stub.clone(), stub));
    }

    if let Ok(nix_rev) = std::env::var("NIX_REV") {
        let info = format!("nix!{nix_rev:.7}");
        return Some((nix_rev, info));
    }

    // in the future we'd get the commit topic
    // instead of git branches here, would be better
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
fn get_from_git() -> Option<(String, String)> {
    let is_empty = sh(["git", "diff", "--shortstat"])?.is_empty();
    let commit_id = sh(["git", "rev-parse", "HEAD"])?;
    let branch = sh([
        "git",
        "name-rev",
        "--name-only",
        "--refs=heads/*",
        "--refs=tags/*",
        "HEAD",
    ])?;

    let branch = branch
        .strip_suffix("~1")
        .or_else(|| branch.strip_suffix("^0"))
        .filter(|b| *b != "undefined")
        .unwrap_or_default();

    Some(format(!is_empty, &commit_id, branch))
}

fn emit_build_info() {
    // either git or colocated
    if Path::new(".git/HEAD").exists() {
        println!("cargo::rerun-if-changed=.git/HEAD");
    }
    // any jj op change
    if Path::new(".jj/repo/op_heads/heads").exists() {
        println!("cargo::rerun-if-changed=.jj/repo/op_heads/heads");
    }
    println!("cargo::rerun-if-env-changed=BUILD_COMMIT");

    let (commit, info) = get_from_jj()
        .or_else(get_from_git)
        .expect("Building without jj or git installed, or not in a repo");

    println!("cargo::rustc-env=BUILD_COMMIT={commit}");
    println!("cargo::rustc-env=BUILD_INFO={info}");
}

fn embed_windows_resource() {
    if env::var_os("CARGO_CFG_WINDOWS").is_some() {
        if let Err(e) = WindowsResource::new().set_icon("res/icon.ico").compile() {
            eprintln!("Failed to embed Windows resource: {e}");
            std::process::exit(1);
        }
    }
}

fn main() {
    emit_build_info();
    embed_windows_resource();
}
