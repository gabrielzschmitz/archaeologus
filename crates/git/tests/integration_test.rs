use std::path::Path;

use archaeologist_git::{
    blame_file, clone_repository, diff_commit, walk_commits, CloneOptions, FileStatus, WalkFilter,
};
use tempfile::TempDir;

fn build_repo(n: usize) -> (TempDir, Vec<String>) {
    let dir = TempDir::new().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    let sig = git2::Signature::now("Integration User", "int@test.io").unwrap();

    let mut shas = Vec::new();
    let mut parent: Option<git2::Oid> = None;

    for i in 0..n {
        let fname = format!("file{i}.txt");
        let fpath = dir.path().join(&fname);
        std::fs::write(&fpath, format!("line1\nline{i}\n")).unwrap();

        let mut idx = repo.index().unwrap();
        idx.add_path(Path::new(&fname)).unwrap();
        idx.write().unwrap();
        let tree_id = idx.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();

        let parents: Vec<git2::Commit> =
            parent.map(|o| repo.find_commit(o).unwrap()).into_iter().collect();
        let prefs: Vec<&git2::Commit> = parents.iter().collect();

        let oid = repo
            .commit(Some("HEAD"), &sig, &sig, &format!("commit {i}"), &tree, &prefs)
            .unwrap();

        parent = Some(oid);
        shas.push(oid.to_string());
    }

    (dir, shas)
}

#[test]
fn integration_clone_local_repo() {
    let (src, _) = build_repo(3);
    let dest = TempDir::new().unwrap();
    let dest_path = dest.path().join("cloned");

    let url = format!("file://{}", src.path().display());
    let result = clone_repository(&url, &dest_path, CloneOptions::default());
    assert!(result.is_ok(), "clone failed: {:?}", result.err());

    let cloned_path = result.unwrap();
    assert!(cloned_path.join(".git").exists(), ".git dir missing after clone");
}

#[test]
fn integration_clone_bad_url_errors() {
    let dest = TempDir::new().unwrap();
    let result = clone_repository(
        "file:///tmp/totally_nonexistent_repo_xyz",
        &dest.path().join("out"),
        CloneOptions::default(),
    );
    assert!(result.is_err());
}

#[test]
fn integration_walk_commits_count() {
    let (dir, shas) = build_repo(5);
    let commits = walk_commits(dir.path(), &WalkFilter::default()).unwrap();
    assert_eq!(commits.len(), shas.len());
}

#[test]
fn integration_walk_commits_chain() {
    let (dir, _) = build_repo(4);
    let commits = walk_commits(dir.path(), &WalkFilter::default()).unwrap();
    for window in commits.windows(2) {
        assert!(
            window[0].parent_shas.contains(&window[1].sha),
            "parent chain broken between {} and {}",
            window[0].sha,
            window[1].sha
        );
    }
}

#[test]
fn integration_walk_commits_author_filter() {
    let (dir, _) = build_repo(3);
    let filter = WalkFilter {
        author: Some("integration user".to_string()),
        ..Default::default()
    };
    let commits = walk_commits(dir.path(), &filter).unwrap();
    assert_eq!(commits.len(), 3);

    let filter_none = WalkFilter {
        author: Some("nobody".to_string()),
        ..Default::default()
    };
    let none = walk_commits(dir.path(), &filter_none).unwrap();
    assert!(none.is_empty());
}

#[test]
fn integration_walk_empty_repo() {
    let dir = TempDir::new().unwrap();
    git2::Repository::init(dir.path()).unwrap();
    let commits = walk_commits(dir.path(), &WalkFilter::default()).unwrap();
    assert!(commits.is_empty());
}

#[test]
fn integration_blame_file_coverage() {
    let (dir, _) = build_repo(1);
    let hunks = blame_file(dir.path(), "file0.txt").unwrap();
    let total_lines: usize = hunks.iter().map(|h| h.line_count).sum();
    assert_eq!(total_lines, 2);
    assert_eq!(hunks[0].author_email, "int@test.io");
}

#[test]
fn integration_blame_missing_file_is_err() {
    let (dir, _) = build_repo(1);
    let result = blame_file(dir.path(), "ghost.txt");
    assert!(result.is_err());
}

#[test]
fn integration_diff_root_commit() {
    let (dir, shas) = build_repo(1);
    let files = diff_commit(dir.path(), &shas[0]).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].status, FileStatus::Added);
}

#[test]
fn integration_diff_subsequent_commit() {
    let (dir, shas) = build_repo(3);
    let files = diff_commit(dir.path(), &shas[2]).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].status, FileStatus::Added);
    assert_eq!(files[0].file_path, "file2.txt");
}

#[test]
fn integration_diff_invalid_sha_is_err() {
    let (dir, _) = build_repo(1);
    let result = diff_commit(dir.path(), "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef");
    assert!(result.is_err());
}

#[test]
#[ignore = "requires network; run with: cargo test -- --ignored"]
fn integration_clone_github_https() {
    let dest = TempDir::new().unwrap();
    let dest_path = dest.path().join("fixtures");

    let result = clone_repository(
        "https://github.com/arypog/fixtures",
        &dest_path,
        CloneOptions::default(),
    );
    assert!(result.is_ok(), "HTTPS clone failed: {:?}", result.err());

    let path = result.unwrap();
    assert!(path.join(".git").exists());

    let commits = walk_commits(&path, &WalkFilter::default()).unwrap();
    assert!(!commits.is_empty(), "expected commits in fixtures repo");
}
