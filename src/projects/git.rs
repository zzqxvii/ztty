//! The worktrees a repository holds, read through `git worktree list --porcelain`.

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Stdio,
};

use anyhow::Context as _;

use super::Worktree;

/// The worktrees `folder`'s repository holds, in git's order; the error names what git said.
pub fn worktrees(folder: &Path) -> anyhow::Result<Vec<Worktree>> {
    let named = format!("projects: `git worktree list` in {}", folder.display());
    let mut command = crate::git::command(folder);
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(
        &mut command,
        crate::git::CREATE_NO_WINDOW,
    );
    let output = command
        .args(["worktree", "list", "--porcelain"])
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("{named} did not run"))?;
    if !output.status.success() {
        let said = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("{named} failed: {}", said.trim());
    }
    Ok(parse(&output.stdout))
}

/// One worktree as git's porcelain lists it.
#[derive(Default)]
struct Record {
    path: Option<OsString>,
    branch: Option<String>,
}

impl Record {
    fn line(&mut self, line: &[u8]) {
        let (key, rest) = match line.iter().position(|byte| *byte == b' ') {
            Some(at) => (&line[..at], &line[at + 1..]),
            None => (line, &line[line.len()..]),
        };
        match key {
            b"worktree" => self.path = Some(path(rest)),
            b"branch" => {
                let name = String::from_utf8_lossy(rest).into_owned();
                self.branch = Some(match name.strip_prefix("refs/heads/") {
                    Some(short) => short.to_string(),
                    None => name,
                });
            }
            _ => {}
        }
    }

    fn finish(&mut self) -> Option<Worktree> {
        let record = std::mem::take(self);
        Some(Worktree {
            path: record.path?.into(),
            branch: record.branch,
            default: false,
        })
    }
}

/// The worktrees a porcelain listing names, the first of them the main checkout.
fn parse(output: &[u8]) -> Vec<Worktree> {
    let mut worktrees = Vec::new();
    let mut record = Record::default();
    for line in output.split(|byte| *byte == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.is_empty() {
            worktrees.extend(record.finish());
        } else {
            record.line(line);
        }
    }
    worktrees.extend(record.finish());
    if let Some(first) = worktrees.first_mut() {
        first.default = true;
    }
    worktrees
}

/// The path a `worktree` line names, unquoted from git's C style.
fn path(rest: &[u8]) -> OsString {
    let bytes = match rest {
        [b'"', body @ .., b'"'] => unquote(body),
        body => body.to_vec(),
    };
    os(&bytes)
}

/// A C-quoted path's bytes, as git quotes a name with spaces, quotes, backslashes or bytes not in UTF-8.
fn unquote(body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(body.len());
    let mut index = 0;
    while index < body.len() {
        if body[index] != b'\\' || index + 1 == body.len() {
            out.push(body[index]);
            index += 1;
            continue;
        }
        index += 1;
        match body[index] {
            b'n' => out.push(b'\n'),
            b't' => out.push(b'\t'),
            b'r' => out.push(b'\r'),
            b'a' => out.push(0x07),
            b'b' => out.push(0x08),
            b'f' => out.push(0x0c),
            b'v' => out.push(0x0b),
            b'\\' => out.push(b'\\'),
            b'"' => out.push(b'"'),
            digit @ b'0'..=b'7' => {
                let mut value = u32::from(digit - b'0');
                let mut taken = 1;
                while taken < 3
                    && index + 1 < body.len()
                    && body[index + 1].is_ascii_digit()
                    && body[index + 1] < b'8'
                {
                    index += 1;
                    taken += 1;
                    value = value * 8 + u32::from(body[index] - b'0');
                }
                out.push(value as u8);
            }
            other => out.push(other),
        }
        index += 1;
    }
    out
}

/// `bytes` as a path, whole off Windows.
#[cfg(unix)]
fn os(bytes: &[u8]) -> OsString {
    use std::os::unix::ffi::OsStringExt;
    OsString::from_vec(bytes.to_vec())
}

#[cfg(not(unix))]
fn os(bytes: &[u8]) -> OsString {
    OsString::from(String::from_utf8_lossy(bytes).into_owned())
}

/// `path` as the platform writes it, so git's forward slashes meet the settings' own.
#[cfg(windows)]
pub fn native(path: PathBuf) -> PathBuf {
    match path.to_str() {
        Some(text) => PathBuf::from(text.replace('/', "\\")),
        None => path,
    }
}

#[cfg(not(windows))]
pub fn native(path: PathBuf) -> PathBuf {
    path
}

#[cfg(test)]
mod tests {
    use std::{fs, process::Command};

    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("ztty-worktrees-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a test folder");
        folder
    }

    fn run(folder: &Path, args: &[&str]) {
        let output = Command::new("git")
            .current_dir(folder)
            .args(args)
            .env("LC_ALL", "C")
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn repository(name: &str) -> PathBuf {
        let folder = scratch(name);
        run(&folder, &["init", "--quiet", "-b", "main"]);
        run(
            &folder,
            &[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "--quiet",
                "--allow-empty",
                "-m",
                "one",
            ],
        );
        folder
    }

    #[test]
    fn a_repository_names_its_checkout_first() {
        let folder = repository("main");
        let found = worktrees(&folder).expect("a repository");
        assert_eq!(found.len(), 1);
        assert!(found[0].default, "the main checkout");
        assert_eq!(found[0].branch.as_deref(), Some("main"));
        assert_eq!(
            found[0].path.file_name(),
            folder.file_name(),
            "the checkout itself"
        );
        fs::remove_dir_all(&folder).expect("cleanup");
    }

    #[test]
    fn a_second_worktree_comes_with_its_branch() {
        let folder = repository("second");
        let other = folder.with_file_name(format!(
            "{}-wt",
            folder.file_name().unwrap().to_string_lossy()
        ));
        let _ = fs::remove_dir_all(&other);
        run(
            &folder,
            &[
                "worktree",
                "add",
                "--quiet",
                "-b",
                "feat",
                other.to_str().unwrap(),
            ],
        );
        let found = worktrees(&folder).expect("a repository");
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(found[0].default);
        assert!(!found[1].default);
        assert_eq!(found[1].branch.as_deref(), Some("feat"));
        assert_eq!(found[1].path.file_name(), other.file_name());
        run(
            &folder,
            &["worktree", "remove", "--force", other.to_str().unwrap()],
        );
        fs::remove_dir_all(&folder).expect("cleanup");
    }

    #[test]
    fn a_plain_folder_names_no_worktree() {
        let folder = scratch("plain");
        let error = worktrees(&folder).expect_err("no repository");
        assert!(error.to_string().contains("worktree list"), "{error}");
        fs::remove_dir_all(&folder).expect("cleanup");
    }

    #[test]
    fn a_quoted_path_comes_back_whole() {
        let listing =
            b"worktree \"/tmp/a b\\303\\251\"\nHEAD 0123456789abcdef\nbranch refs/heads/main\n\n";
        let found = parse(listing);
        assert_eq!(found.len(), 1);
        assert!(
            found[0].path.to_string_lossy().ends_with("a b\u{e9}"),
            "{:?}",
            found[0].path
        );
        assert!(found[0].default);
    }

    #[test]
    fn a_detached_worktree_names_no_branch() {
        let listing = b"worktree /tmp/one\nHEAD abcdef0123\nbranch refs/heads/main\n\nworktree /tmp/two\nHEAD 9876543210\ndetached\nlocked reason\n";
        let found = parse(listing);
        assert_eq!(found.len(), 2);
        assert_eq!(found[1].branch, None);
        assert!(!found[1].default);
    }
}
