use std::path::{Path, PathBuf};

use git2::build::RepoBuilder;
use git2::{Cred, FetchOptions, RemoteCallbacks};
use tracing::{debug, info};

use crate::error::{GitError, Result};

pub type ProgressCallback = Box<dyn Fn(usize, usize, usize) + Send>;

#[derive(Default)]
pub struct CloneOptions {
    /// Optional SSH private-key path. `None` → try the SSH agent / default keys.
    pub ssh_key_path: Option<PathBuf>,
    /// Optional progress callback.
    pub on_progress: Option<ProgressCallback>,
}

impl std::fmt::Debug for CloneOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CloneOptions")
            .field("ssh_key_path", &self.ssh_key_path)
            .field("on_progress", &self.on_progress.is_some())
            .finish()
    }
}

/// # Errors
///
/// Returns [`GitError`] on authentication failure, network error, or if `dest`
/// cannot be created.
pub fn clone_repository(url: &str, dest: &Path, opts: CloneOptions) -> Result<PathBuf> {
    info!("Cloning {} -> {:?}", url, dest);

    let mut callbacks = RemoteCallbacks::new();

    let ssh_key_path = opts.ssh_key_path.clone();
    callbacks.credentials(move |_url, username_from_url, allowed_types| {
        let username = username_from_url.unwrap_or("git");

        if allowed_types.contains(git2::CredentialType::SSH_KEY) {
            if let Ok(cred) = Cred::ssh_key_from_agent(username) {
                return Ok(cred);
            }
            if let Some(key) = &ssh_key_path {
                return Cred::ssh_key(username, None, key, None);
            }
            let home = std::env::var("HOME").unwrap_or_default();
            for name in &["id_ed25519", "id_rsa", "id_ecdsa"] {
                let key = PathBuf::from(&home).join(".ssh").join(name);
                if key.exists() {
                    if let Ok(cred) = Cred::ssh_key(username, None, &key, None) {
                        return Ok(cred);
                    }
                }
            }
        }

        if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
            let user = std::env::var("GIT_USERNAME").unwrap_or_default();
            let pass = std::env::var("GIT_PASSWORD").unwrap_or_default();
            if !user.is_empty() {
                return Cred::userpass_plaintext(&user, &pass);
            }
        }

        if allowed_types.contains(git2::CredentialType::DEFAULT) {
            return Cred::default();
        }

        Err(git2::Error::from_str("no suitable credentials available"))
    });

    if let Some(cb) = opts.on_progress {
        callbacks.transfer_progress(move |progress| {
            cb(
                progress.received_objects(),
                progress.total_objects(),
                progress.received_bytes(),
            );
            true
        });
    }

    let mut fetch_opts = FetchOptions::new();
    fetch_opts.remote_callbacks(callbacks);

    let repo = RepoBuilder::new()
        .fetch_options(fetch_opts)
        .clone(url, dest)
        .map_err(|e| {
            debug!("clone failed: {e}");
            if e.message().contains("auth")
                || e.message().contains("authentication")
                || e.message().contains("credentials")
            {
                GitError::AuthFailed(url.to_string())
            } else {
                GitError::Git2(e)
            }
        })?;

    let path = repo.path().parent().unwrap_or(repo.path()).to_path_buf();
    info!("Cloned successfully -> {:?}", path);
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_local_repo() -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();

        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        let mut index = repo.index().unwrap();
        let path = dir.path().join("README.md");
        std::fs::write(&path, "hello").unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
            .unwrap();

        let src = dir.path().to_path_buf();
        (dir, src)
    }

    #[test]
    fn clone_local_repo_succeeds() {
        let (_src_dir, src) = make_local_repo();
        let dest_dir = TempDir::new().unwrap();
        let dest = dest_dir.path().join("clone");

        let url = format!("file://{}", src.display());
        let result = clone_repository(&url, &dest, CloneOptions::default());
        assert!(result.is_ok(), "clone failed: {:?}", result.err());
        assert!(dest.join(".git").exists());
    }

    #[test]
    fn clone_progress_callback_is_invoked() {
        let (_src_dir, src) = make_local_repo();
        let dest_dir = TempDir::new().unwrap();
        let dest = dest_dir.path().join("clone");

        let url = format!("file://{}", src.display());
        let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called_clone = called.clone();
        let opts = CloneOptions {
            on_progress: Some(Box::new(move |_, _, _| {
                called_clone.store(true, std::sync::atomic::Ordering::Relaxed);
            })),
            ..Default::default()
        };
        let _ = clone_repository(&url, &dest, opts);
    }

    #[test]
    fn clone_invalid_url_returns_error() {
        let dest_dir = TempDir::new().unwrap();
        let dest = dest_dir.path().join("clone");
        let result = clone_repository("file:///nonexistent/path/repo", &dest, CloneOptions::default());
        assert!(result.is_err());
    }
}
