use std::path::Path;

use git2::Repository;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::error::{GitError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameHunk {
    /// SHA of the commit that last modified these lines.
    pub commit_sha: String,
    pub author_name: String,
    pub author_email: String,
    /// 1-based index of the first line in this hunk (in the final file).
    pub start_line: usize,
    /// Number of lines in this hunk.
    pub line_count: usize,
}

/// # Errors
///
/// Returns [`GitError`] when the repo is invalid, HEAD is absent, or the
/// blame computation fails (e.g. the file does not exist in HEAD).
pub fn blame_file(repo_path: &Path, file_path: &str) -> Result<Vec<BlameHunk>> {
    info!("Blaming {:?} in {:?}", file_path, repo_path);

    let repo = Repository::open(repo_path)
        .map_err(|_| GitError::RepoNotFound(repo_path.display().to_string()))?;

    if repo.head().is_err() {
        debug!("repository has no HEAD; returning empty blame");
        return Ok(vec![]);
    }

    let blame = repo
        .blame_file(Path::new(file_path), None)
        .map_err(|e| {
            if e.message().contains("not found") || e.message().contains("does not exist") {
                GitError::FileNotFound(file_path.to_string())
            } else {
                GitError::Git2(e)
            }
        })?;

    let mut hunks: Vec<BlameHunk> = Vec::new();

    for raw in blame.iter() {
        let commit_sha = raw.final_commit_id().to_string();
        let sig = raw.final_signature();
        let author_name = sig
            .as_ref()
            .and_then(|s| s.name().ok())
            .unwrap_or("")
            .to_string();
        let author_email = sig
            .as_ref()
            .and_then(|s| s.email().ok())
            .unwrap_or("")
            .to_string();
        let start_line = raw.final_start_line(); // 1-based
        let line_count = raw.lines_in_hunk();

        if let Some(last) = hunks.last_mut() {
            if last.commit_sha == commit_sha
                && last.start_line + last.line_count == start_line
            {
                last.line_count += line_count;
                continue;
            }
        }

        hunks.push(BlameHunk {
            commit_sha,
            author_name,
            author_email,
            start_line,
            line_count,
        });
    }

    info!("Blame produced {} hunks", hunks.len());
    Ok(hunks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_repo_with_file(content: &str) -> (TempDir, String) {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();

        let path = dir.path().join("hello.txt");
        std::fs::write(&path, content).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("hello.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "add hello.txt", &tree, &[])
            .unwrap();

        (dir, "hello.txt".to_string())
    }

    #[test]
    fn blame_single_commit_file() {
        let (dir, file) = make_repo_with_file("line one\nline two\nline three\n");
        let hunks = blame_file(dir.path(), &file).unwrap();
        assert!(!hunks.is_empty());
        // All lines should come from the single commit
        assert_eq!(hunks[0].author_name, "Test User");
        let total_lines: usize = hunks.iter().map(|h| h.line_count).sum();
        assert_eq!(total_lines, 3);
    }

    #[test]
    fn blame_empty_repo_returns_empty() {
        let dir = TempDir::new().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        let result = blame_file(dir.path(), "nonexistent.txt").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn blame_missing_file_returns_error() {
        let (dir, _) = make_repo_with_file("content\n");
        let result = blame_file(dir.path(), "does_not_exist.txt");
        assert!(result.is_err());
    }

    #[test]
    fn blame_invalid_repo_returns_error() {
        let result = blame_file(Path::new("/nonexistent"), "file.txt");
        assert!(matches!(result, Err(GitError::RepoNotFound(_))));
    }
}