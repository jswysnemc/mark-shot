// Shots that were only copied without saving are still temporarly
// saved so the notification can preview them.
//
// Only for daemons that need a separate file for a proper preview,
// like Plasma one, the rest directly take the pixels
//
// set to Read-only, and kept in XDG_RUNTIME_DIR, which is wiped on logout.

use std::io;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};

const BUDGET: u64 = 50 * 1024 * 1024;

pub fn write(png: &[u8]) -> io::Result<PathBuf> {
    let dir = dirs::runtime_dir()
        .ok_or_else(|| io::Error::other("XDG_RUNTIME_DIR is not set"))?
        .join("mark-shot")
        .join("previews");
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&dir)?;
    let format = &crate::config::get().save.filename_format;
    let stem = chrono::Local::now().format(format).to_string();
    let path = crate::utils::write_unique(&dir, &stem, png, 0o400)?;
    prune(&dir, &path, BUDGET);
    Ok(path)
}

/// deletes older previews to fit in the 'budget'
/// (logging out deletes them already, so its just to be safe)
fn prune(dir: &Path, keep: &Path, budget: u64) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<_> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let meta = entry.metadata().ok()?;
            let path = entry.path();
            let png = path.extension().is_some_and(|ext| ext == "png");
            (meta.is_file() && png).then_some((path, meta.modified().ok()?, meta.len()))
        })
        .collect();
    files.sort_by_key(|&(_, modified, _)| std::cmp::Reverse(modified));

    let mut total = 0;
    for (path, _, len) in files {
        total += len;
        // new file is never removed, to not break the notification
        if total > budget
            && path != keep
            && let Err(e) = std::fs::remove_file(&path)
            && e.kind() != io::ErrorKind::NotFound
        {
            log::info!("can't remove the preview {}: {e}", path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::prune;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime};

    fn file(dir: &Path, name: &str, len: usize, age: u64) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, vec![0; len]).unwrap();
        let time = SystemTime::now() - Duration::from_secs(age);
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(time)
            .unwrap();
        path
    }

    #[test]
    fn prune_removes_the_oldest_over_budget() {
        let dir = std::env::temp_dir().join(format!("lumine-prune-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let old = file(&dir, "old.png", 40, 30);
        let mid = file(&dir, "mid.png", 40, 20);
        let new = file(&dir, "new.png", 40, 10);
        let other = file(&dir, "other.txt", 1000, 40);

        prune(&dir, &new, 100);

        assert!(!old.exists());
        assert!(mid.exists() && new.exists() && other.exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
