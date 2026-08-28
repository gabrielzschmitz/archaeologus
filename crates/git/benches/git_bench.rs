use std::path::Path;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use archaeologist_git::{blame_file, diff_commit, walk_commits, WalkFilter};
use tempfile::TempDir;

fn build_repo(n: usize) -> TempDir {
    let dir = TempDir::new().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    let sig = git2::Signature::now("Bench User", "bench@bench.io").unwrap();

    let mut parent: Option<git2::Oid> = None;
    for i in 0..n {
        let fname = format!("file{i}.txt");
        let content = format!("line1\nline2\nvalue={i}\n");
        std::fs::write(dir.path().join(&fname), &content).unwrap();

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
    }
    dir
}

fn build_large_file_repo(lines: usize) -> (TempDir, String) {
    let dir = TempDir::new().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    let sig = git2::Signature::now("Bench User", "bench@bench.io").unwrap();

    let content: String = (0..lines).fold(String::new(), |mut s, i| {
        use std::fmt::Write;
        let _ = writeln!(s, "line {i}");
        s
    });
    std::fs::write(dir.path().join("big.txt"), &content).unwrap();

    let mut idx = repo.index().unwrap();
    idx.add_path(Path::new("big.txt")).unwrap();
    idx.write().unwrap();
    let tree_id = idx.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "add big file", &tree, &[])
        .unwrap();

    let sha = repo.head().unwrap().peel_to_commit().unwrap().id().to_string();
    (dir, sha)
}

fn bench_walk_commits(c: &mut Criterion) {
    let mut group = c.benchmark_group("walk_commits");

    for &n in &[100_usize, 1_000, 5_000] {
        let dir = build_repo(n);
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| walk_commits(dir.path(), &WalkFilter::default()).unwrap());
        });
    }

    group.finish();
}

fn bench_blame(c: &mut Criterion) {
    let mut group = c.benchmark_group("blame_file");

    for &lines in &[500_usize, 2_000, 10_000] {
        let (dir, _sha) = build_large_file_repo(lines);
        group.throughput(Throughput::Elements(lines as u64));
        group.bench_with_input(BenchmarkId::from_parameter(lines), &lines, |b, _| {
            b.iter(|| blame_file(dir.path(), "big.txt").unwrap());
        });
    }

    group.finish();
}

fn bench_diff(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_commit");

    let dir = build_repo(50);
    let repo = git2::Repository::open(dir.path()).unwrap();
    let sha = repo.head().unwrap().peel_to_commit().unwrap().id().to_string();

    group.bench_function("last_commit", |b| {
        b.iter(|| diff_commit(dir.path(), &sha).unwrap());
    });

    group.finish();
}

criterion_group!(benches, bench_walk_commits, bench_blame, bench_diff);
criterion_main!(benches);
