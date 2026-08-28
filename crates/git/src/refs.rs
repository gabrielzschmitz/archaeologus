//! Extract branches and tags from a local git repository.

use std::path::Path;

use git2::Repository;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::{GitError, Result};

/// A local or remote branch reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub name: String,
    /// SHA of the commit the branch points to.
    pub head_sha: String,
    /// True when this is the repository's default branch (HEAD).
    pub is_default: bool,
}

/// A lightweight or annotated tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagInfo {
    pub name: String,
    /// SHA of the tagged object (peel to commit when annotated).
    pub target_sha: String,
}

/// List every branch in `repo_path` — both local branches and every remote
/// tracking ref (e.g. `refs/remotes/origin/*`).
///
/// For cloned repositories this is the only way to discover branches that were
/// never checked out locally (e.g. `brain-f#` in a shallow clone that only
/// checked out `main`).
///
/// # Errors
/// Returns [`GitError`] if the path is not a valid git repository.
pub fn list_branches(repo_path: &Path) -> Result<Vec<BranchInfo>> {
    info!("Listing branches in {:?}", repo_path);
    let repo = Repository::open(repo_path)
        .map_err(|_| GitError::RepoNotFound(repo_path.display().to_string()))?;

    // Try to fetch all remote branches so we see everything the server has.
    fetch_all_remotes(&repo);

    // Determine the SHA HEAD currently points to (marks the default branch).
    let head_sha = repo
        .head()
        .ok()
        .and_then(|h| h.target())
        .map(|o| o.to_string());

    // Also resolve what the remote considers its default branch via
    // `refs/remotes/<remote>/HEAD`.  We use this to mark remote branches.
    let remote_default_sha = remote_head_sha(&repo);

    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut branches = Vec::new();

    // ── 1. Local branches ────────────────────────────────────────────────────
    for branch_result in repo.branches(Some(git2::BranchType::Local))? {
        let (branch, _) = branch_result?;
        let name = match branch.name() {
            Ok(Some(n)) => n.to_string(),
            _ => continue,
        };
        let sha = match branch.get().target() {
            Some(oid) => oid.to_string(),
            None => continue,
        };
        let is_default = head_sha.as_deref() == Some(sha.as_str())
            || remote_default_sha.as_deref() == Some(sha.as_str());
        seen.insert(name.clone());
        branches.push(BranchInfo {
            name,
            head_sha: sha,
            is_default,
        });
    }

    // ── 2. Remote tracking refs (refs/remotes/<remote>/<branch>) ─────────────
    // Walk every reference, filter to refs/remotes/*, strip the remote prefix,
    // skip synthetic "HEAD" entries.
    let refs = repo.references()?;
    for reference in refs.flatten() {
        // reference.name() returns Result<&str, git2::Error>.
        let refname = match reference.name() {
            Ok(n) => n.to_string(),
            Err(_) => continue,
        };

        // Only remote tracking refs.
        if !refname.starts_with("refs/remotes/") {
            continue;
        }

        // Strip "refs/remotes/<remote>/" prefix to get the branch name.
        // e.g. "refs/remotes/origin/brain-f#" → "brain-f#"
        let after_remotes = &refname["refs/remotes/".len()..];
        let branch_name = match after_remotes.find('/') {
            Some(idx) => after_remotes[idx + 1..].to_string(),
            None => continue, // "refs/remotes/origin" without a slash — skip
        };

        // Skip the synthetic HEAD pointer git writes for remote tracking.
        if branch_name == "HEAD" {
            continue;
        }

        // Skip if we already have this name from a local branch.
        if seen.contains(&branch_name) {
            continue;
        }

        let sha = match reference.target() {
            Some(oid) => oid.to_string(),
            None => continue,
        };

        let is_default = remote_default_sha.as_deref() == Some(sha.as_str());
        seen.insert(branch_name.clone());
        branches.push(BranchInfo {
            name: branch_name,
            head_sha: sha,
            is_default,
        });
    }

    info!("Found {} branches", branches.len());
    Ok(branches)
}

/// Attempt to fetch all configured remotes.  Errors are silently ignored —
/// we're in a read path and network failures must not abort indexing.
fn fetch_all_remotes(repo: &Repository) {
    let Ok(remotes) = repo.remotes() else {
        return;
    };
    // git2::StringArray::iter() yields Result<Option<&str>, git2::Error>.
    let names: Vec<String> = remotes
        .iter()
        .filter_map(|r| r.ok().flatten().map(str::to_string))
        .collect();
    for name in &names {
        if let Ok(mut remote) = repo.find_remote(name) {
            let mut fetch_opts = git2::FetchOptions::new();
            fetch_opts.download_tags(git2::AutotagOption::All);
            let refspec = format!("+refs/heads/*:refs/remotes/{name}/*");
            let _ = remote.fetch(&[refspec.as_str()], Some(&mut fetch_opts), None);
        }
    }
}

/// Resolve the SHA that `refs/remotes/<remote>/HEAD` points to, if present.
fn remote_head_sha(repo: &Repository) -> Option<String> {
    let remotes = repo.remotes().ok()?;
    // StringArray::iter() yields Result<Option<&str>, git2::Error>.
    let first: String = remotes
        .iter()
        .find_map(|r| r.ok().flatten().map(str::to_string))?;
    let ref_path = format!("refs/remotes/{first}/HEAD");
    let reference = repo.find_reference(&ref_path).ok()?;
    // HEAD is a symbolic ref; resolve it to the target branch's OID.
    let resolved = reference.resolve().ok()?;
    resolved.target().map(|o| o.to_string())
}

/// List every tag in `repo_path`.
///
/// # Errors
/// Returns [`GitError`] if the path is not a valid git repository.
pub fn list_tags(repo_path: &Path) -> Result<Vec<TagInfo>> {
    info!("Listing tags in {:?}", repo_path);
    let repo = Repository::open(repo_path)
        .map_err(|_| GitError::RepoNotFound(repo_path.display().to_string()))?;

    let tag_names = repo.tag_names(None)?;
    let mut tags = Vec::new();

    for name_opt in &tag_names {
        let name = match name_opt {
            Ok(Some(n)) => n.to_string(),
            _ => continue,
        };

        // Resolve the tag reference to a SHA.
        let refname = format!("refs/tags/{name}");
        let Ok(reference) = repo.find_reference(&refname) else {
            continue;
        };

        // Peel annotated tags down to the underlying commit object.
        let sha = match reference.peel_to_commit() {
            Ok(commit) => commit.id().to_string(),
            Err(_) => match reference.target() {
                Some(oid) => oid.to_string(),
                None => continue,
            },
        };

        tags.push(TagInfo {
            name,
            target_sha: sha,
        });
    }

    info!("Found {} tags", tags.len());
    Ok(tags)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::TempDir;

    fn make_repo_with_commit(msg: &str) -> (TempDir, git2::Oid) {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "t@t.com").unwrap();
        let path = dir.path().join("f.txt");
        std::fs::write(&path, msg).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("f.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let oid = repo
            .commit(Some("HEAD"), &sig, &sig, msg, &tree, &[])
            .unwrap();
        (dir, oid)
    }

    #[test]
    fn list_branches_returns_main_branch() {
        let (dir, _) = make_repo_with_commit("init");
        let branches = list_branches(dir.path()).unwrap();
        assert!(!branches.is_empty());
        // git init creates "master" or "main" depending on config
        let names: Vec<&str> = branches.iter().map(|b| b.name.as_str()).collect();
        assert!(
            names.contains(&"master") || names.contains(&"main"),
            "expected master or main, got {names:?}"
        );
        // At least one branch should be the default (local repos have no
        // remote HEAD so is_default is set via the local HEAD sha).
        let defaults = branches.iter().filter(|b| b.is_default).count();
        assert!(defaults <= 1, "at most 1 default expected, got {defaults}");
    }

    #[test]
    fn list_branches_on_non_repo_returns_error() {
        let dir = TempDir::new().unwrap();
        assert!(list_branches(dir.path()).is_err());
    }

    #[test]
    fn list_tags_empty_when_no_tags() {
        let (dir, _) = make_repo_with_commit("init");
        let tags = list_tags(dir.path()).unwrap();
        assert!(tags.is_empty());
    }

    #[test]
    fn list_tags_returns_lightweight_tag() {
        let (dir, oid) = make_repo_with_commit("init");
        let repo = git2::Repository::open(dir.path()).unwrap();
        let obj = repo.find_object(oid, None).unwrap();
        repo.tag_lightweight("v1.0", &obj, false).unwrap();

        let tags = list_tags(dir.path()).unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "v1.0");
    }
}
