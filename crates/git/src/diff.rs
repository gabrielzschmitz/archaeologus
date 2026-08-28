use std::collections::HashMap;
use std::path::Path;

use git2::{Delta, DiffOptions, Repository};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::error::{GitError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Other,
}

impl From<Delta> for FileStatus {
    fn from(d: Delta) -> Self {
        match d {
            Delta::Added => Self::Added,
            Delta::Modified => Self::Modified,
            Delta::Deleted => Self::Deleted,
            Delta::Renamed => Self::Renamed,
            Delta::Copied => Self::Copied,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitDiffFile {
    /// Current path of the file (after the commit).
    pub file_path: String,
    pub status: FileStatus,
    /// Number of added lines (context lines excluded).
    pub additions: usize,
    /// Number of deleted lines (context lines excluded).
    pub deletions: usize,
    /// Previous path, populated only for renames/copies.
    pub old_path: Option<String>,
}

/// # Errors
///
/// Returns [`GitError`] when the repo is invalid, `sha` is not found, or the
/// diff computation fails.
pub fn diff_commit(repo_path: &Path, sha: &str) -> Result<Vec<CommitDiffFile>> {
    info!("Diffing {} in {:?}", sha, repo_path);

    let repo = Repository::open(repo_path)
        .map_err(|_| GitError::RepoNotFound(repo_path.display().to_string()))?;

    let oid = git2::Oid::from_str(sha).map_err(|_| GitError::CommitNotFound(sha.to_string()))?;

    let commit = repo
        .find_commit(oid)
        .map_err(|_| GitError::CommitNotFound(sha.to_string()))?;

    let commit_tree = commit.tree()?;

    let parent_tree = if commit.parent_count() > 0 {
        debug!("using first parent as diff base");
        Some(commit.parent(0)?.tree()?)
    } else {
        debug!("root commit; diffing against empty tree");
        None
    };

    let mut diff_opts = DiffOptions::new();
    diff_opts.ignore_whitespace(false);

    let diff = repo.diff_tree_to_tree(
        parent_tree.as_ref(),
        Some(&commit_tree),
        Some(&mut diff_opts),
    )?;

    let mut files: Vec<CommitDiffFile> = Vec::new();

    diff.foreach(
        &mut |delta, _progress| {
            let status = FileStatus::from(delta.status());
            let file_path = delta
                .new_file()
                .path()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            let old_path = if matches!(status, FileStatus::Renamed | FileStatus::Copied) {
                delta
                    .old_file()
                    .path()
                    .map(|p| p.to_string_lossy().into_owned())
            } else {
                None
            };
            files.push(CommitDiffFile {
                file_path,
                status,
                additions: 0,
                deletions: 0,
                old_path,
            });
            true
        },
        None,
        None,
        None,
    )?;

    let mut line_counts: HashMap<String, (usize, usize)> = HashMap::new();
    diff.foreach(
        &mut |_, _| true,
        None,
        None,
        Some(&mut |delta, _hunk, line| {
            let path = delta
                .new_file()
                .path()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            let entry = line_counts.entry(path).or_insert((0, 0));
            match line.origin() {
                '+' => entry.0 += 1,
                '-' => entry.1 += 1,
                _ => {}
            }
            true
        }),
    )?;

    for f in &mut files {
        if let Some(&(add, del)) = line_counts.get(&f.file_path) {
            f.additions = add;
            f.deletions = del;
        }
    }

    info!("Diff produced {} file entries", files.len());
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn init_repo() -> (TempDir, git2::Repository) {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        (dir, repo)
    }

    fn commit_file(
        repo: &git2::Repository,
        dir: &Path,
        filename: &str,
        content: &str,
        parent: Option<git2::Oid>,
        message: &str,
    ) -> git2::Oid {
        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        let path = dir.join(filename);
        std::fs::write(&path, content).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(filename)).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let parents: Vec<git2::Commit> = parent
            .map(|oid| repo.find_commit(oid).unwrap())
            .into_iter()
            .collect();
        let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)
            .unwrap()
    }

    #[test]
    fn diff_root_commit_shows_added_file() {
        let (dir, repo) = init_repo();
        let oid = commit_file(&repo, dir.path(), "a.txt", "hello\nworld\n", None, "init");

        let files = diff_commit(dir.path(), &oid.to_string()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, FileStatus::Added);
        assert_eq!(files[0].file_path, "a.txt");
        assert_eq!(files[0].additions, 2);
        assert_eq!(files[0].deletions, 0);
    }

    #[test]
    fn diff_second_commit_shows_modified_file() {
        let (dir, repo) = init_repo();
        let oid1 = commit_file(&repo, dir.path(), "a.txt", "hello\n", None, "init");
        let oid2 = commit_file(
            &repo,
            dir.path(),
            "a.txt",
            "hello\nworld\n",
            Some(oid1),
            "update",
        );

        let files = diff_commit(dir.path(), &oid2.to_string()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, FileStatus::Modified);
        assert_eq!(files[0].additions, 1);
        assert_eq!(files[0].deletions, 0);
    }

    #[test]
    fn diff_deleted_file() {
        let (dir, repo) = init_repo();
        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();

        // commit 1: add file
        let oid1 = commit_file(&repo, dir.path(), "b.txt", "bye\n", None, "add");

        // commit 2: delete file
        std::fs::remove_file(dir.path().join("b.txt")).unwrap();
        let mut index = repo.index().unwrap();
        index.remove_path(Path::new("b.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let parent = repo.find_commit(oid1).unwrap();
        let oid2 = repo
            .commit(Some("HEAD"), &sig, &sig, "delete", &tree, &[&parent])
            .unwrap();

        let files = diff_commit(dir.path(), &oid2.to_string()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, FileStatus::Deleted);
    }

    #[test]
    fn diff_unknown_sha_returns_error() {
        let (dir, _repo) = init_repo();
        let result = diff_commit(dir.path(), "0000000000000000000000000000000000000000");
        assert!(result.is_err());
    }

    #[test]
    fn diff_invalid_repo_returns_error() {
        let result = diff_commit(
            Path::new("/nonexistent"),
            "0000000000000000000000000000000000000000",
        );
        assert!(matches!(result, Err(GitError::RepoNotFound(_))));
    }
}
