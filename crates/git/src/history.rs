use std::path::Path;

use chrono::{DateTime, TimeZone, Utc};
use git2::{Repository, Sort};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::error::{GitError, Result};

/// Lightweight commit representation returned by [`walk_commits`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub sha: String,
    pub author_name: String,
    pub author_email: String,
    pub author_date: DateTime<Utc>,
    pub committer_name: String,
    pub committer_email: String,
    pub committer_date: DateTime<Utc>,
    pub message: String,
    pub parent_shas: Vec<String>,
}

/// Filters applied to [`walk_commits`].
#[derive(Debug, Default, Clone)]
pub struct WalkFilter {
    /// Only include commits at or after this date.
    pub since: Option<DateTime<Utc>>,
    /// Only include commits strictly before this date.
    pub until: Option<DateTime<Utc>>,
    /// Case-insensitive substring match against author name or e-mail.
    pub author: Option<String>,
}

/// Walk every reachable commit in `repo_path`, newest-first, applying `filter`.
///
/// # Errors
///
/// Returns [`GitError`] if the path is not a valid git repository or history
/// traversal fails.
pub fn walk_commits(repo_path: &Path, filter: &WalkFilter) -> Result<Vec<CommitInfo>> {
    info!("Walking commits in {:?}", repo_path);

    let repo = Repository::open(repo_path)
        .map_err(|_| GitError::RepoNotFound(repo_path.display().to_string()))?;

    let mut revwalk = repo.revwalk()?;
    revwalk.set_sorting(Sort::TIME | Sort::TOPOLOGICAL)?;

    if revwalk.push_head().is_err() {
        debug!("repository has no HEAD; returning empty commit list");
        return Ok(vec![]);
    }

    let author_lower = filter.author.as_deref().map(str::to_lowercase);

    let mut commits = Vec::new();
    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;

        let author = commit.author();
        let committer = commit.committer();

        let author_date = git_time_to_utc(author.when());
        let committer_date = git_time_to_utc(committer.when());

        if let Some(since) = filter.since {
            if author_date < since {
                continue;
            }
        }
        if let Some(until) = filter.until {
            if author_date >= until {
                continue;
            }
        }

        if let Some(needle) = &author_lower {
            let name_lc = author.name().unwrap_or("").to_lowercase();
            let email_lc = author.email().unwrap_or("").to_lowercase();
            if !name_lc.contains(needle.as_str()) && !email_lc.contains(needle.as_str()) {
                continue;
            }
        }

        let parent_shas = commit
            .parents()
            .map(|p| p.id().to_string())
            .collect::<Vec<_>>();

        commits.push(CommitInfo {
            sha: oid.to_string(),
            author_name: author.name().unwrap_or("").to_string(),
            author_email: author.email().unwrap_or("").to_string(),
            author_date,
            committer_name: committer.name().unwrap_or("").to_string(),
            committer_email: committer.email().unwrap_or("").to_string(),
            committer_date,
            message: commit.message().unwrap_or("").to_string(),
            parent_shas,
        });
    }

    info!("Found {} commits", commits.len());
    Ok(commits)
}

fn git_time_to_utc(t: git2::Time) -> DateTime<Utc> {
    Utc.timestamp_opt(t.seconds(), 0)
        .single()
        .unwrap_or_else(Utc::now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_repo_n_commits(n: usize, author_name: &str, author_email: &str) -> TempDir {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now(author_name, author_email).unwrap();

        let mut parent_commit: Option<git2::Oid> = None;
        for i in 0..n {
            let path = dir.path().join(format!("file{i}.txt"));
            std::fs::write(&path, format!("content {i}")).unwrap();
            let mut index = repo.index().unwrap();
            index.add_path(Path::new(&format!("file{i}.txt"))).unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            let parents: Vec<git2::Commit> = parent_commit
                .map(|oid| repo.find_commit(oid).unwrap())
                .into_iter()
                .collect();
            let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
            let oid = repo
                .commit(
                    Some("HEAD"),
                    &sig,
                    &sig,
                    &format!("commit {i}"),
                    &tree,
                    &parent_refs,
                )
                .unwrap();
            parent_commit = Some(oid);
        }
        dir
    }

    #[test]
    fn walk_empty_repo_returns_empty() {
        let dir = TempDir::new().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        let result = walk_commits(dir.path(), &WalkFilter::default()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn walk_single_commit() {
        let dir = make_repo_n_commits(1, "Alice", "alice@example.com");
        let commits = walk_commits(dir.path(), &WalkFilter::default()).unwrap();
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].author_name, "Alice");
        assert!(commits[0].parent_shas.is_empty());
    }

    #[test]
    fn walk_multiple_commits_newest_first() {
        let dir = make_repo_n_commits(3, "Bob", "bob@example.com");
        let commits = walk_commits(dir.path(), &WalkFilter::default()).unwrap();
        assert_eq!(commits.len(), 3);
        assert_eq!(commits[1].sha, commits[0].parent_shas[0]);
    }

    #[test]
    fn walk_filter_by_author_name() {
        let dir = make_repo_n_commits(2, "Carol", "carol@example.com");
        let filter = WalkFilter {
            author: Some("carol".to_string()),
            ..Default::default()
        };
        let commits = walk_commits(dir.path(), &filter).unwrap();
        assert_eq!(commits.len(), 2);

        let filter_miss = WalkFilter {
            author: Some("dave".to_string()),
            ..Default::default()
        };
        let none = walk_commits(dir.path(), &filter_miss).unwrap();
        assert!(none.is_empty());
    }

    #[test]
    fn walk_filter_by_author_email() {
        let dir = make_repo_n_commits(2, "Eve", "eve@corp.io");
        let filter = WalkFilter {
            author: Some("corp.io".to_string()),
            ..Default::default()
        };
        let commits = walk_commits(dir.path(), &filter).unwrap();
        assert_eq!(commits.len(), 2);
    }

    #[test]
    fn walk_invalid_path_returns_error() {
        let result = walk_commits(Path::new("/nonexistent"), &WalkFilter::default());
        assert!(matches!(result, Err(GitError::RepoNotFound(_))));
    }
}
