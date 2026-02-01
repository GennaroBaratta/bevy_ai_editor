use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

// The global registry of locked files
static LOCKED_FILES: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn get_locked_files() -> &'static Mutex<HashSet<String>> {
    LOCKED_FILES.get_or_init(|| Mutex::new(HashSet::new()))
}

// The guard returned when a lock is acquired
pub struct FileLockGuard {
    path: String,
}

impl Drop for FileLockGuard {
    fn drop(&mut self) {
        if let Ok(mut locked) = get_locked_files().lock() {
            locked.remove(&self.path);
        }
    }
}

pub fn acquire_lock(path: &str) -> Result<FileLockGuard> {
    let path_buf = Path::new(path);

    // Canonicalize the path to ensure uniqueness.
    // If the file exists, fs::canonicalize will return the absolute path with symlinks resolved.
    // If it doesn't exist, we fallback to absolute path resolution relative to CWD.
    let canonical_path = if let Ok(p) = path_buf.canonicalize() {
        p.to_string_lossy().into_owned()
    } else {
        // Fallback for new files or when canonicalize fails
        if path_buf.is_absolute() {
            path_buf.to_string_lossy().into_owned()
        } else {
            std::env::current_dir()
                .map(|cwd| cwd.join(path).to_string_lossy().into_owned())
                .unwrap_or_else(|_| path.to_string())
        }
    };

    let locked_files = get_locked_files();

    // Spin loop with backoff
    loop {
        {
            let mut set = locked_files
                .lock()
                .map_err(|_| anyhow::anyhow!("Global lock registry poisoned"))?;
            if !set.contains(&canonical_path) {
                set.insert(canonical_path.clone());
                return Ok(FileLockGuard {
                    path: canonical_path,
                });
            }
        }
        // Wait a bit before retrying.
        // Simple backoff: 50ms. Could be randomized or exponential in a more complex system.
        thread::sleep(Duration::from_millis(50));
    }
}
