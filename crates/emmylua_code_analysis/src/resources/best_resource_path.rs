use sha2::{Digest, Sha256};
use std::path::PathBuf;

pub fn get_best_resources_dir() -> PathBuf {
    let cache_key = format!("{}-{}", env!("CARGO_PKG_VERSION"), resource_fingerprint());
    if cfg!(debug_assertions) {
        return exe_dir().join("resources").join(cache_key);
    }

    if cfg!(target_os = "windows") {
        // On Windows, try LOCALAPPDATA first
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            PathBuf::from(local_app_data)
                .join("emmylua_ls")
                .join("resources")
                .join(&cache_key)
        } else {
            // Fall back to the directory next to the executable
            exe_dir().join("resources").join(&cache_key)
        }
    } else {
        // On non-Windows platforms, try XDG_DATA_HOME first
        if let Ok(xdg_data_home) = std::env::var("XDG_DATA_HOME") {
            PathBuf::from(xdg_data_home)
                .join("emmylua_ls")
                .join("resources")
        } else {
            // If XDG_DATA_HOME is not set, use default XDG path
            if let Ok(home) = std::env::var("HOME") {
                PathBuf::from(home)
                    .join(".local")
                    .join("share")
                    .join("emmylua_ls")
                    .join("resources")
                    .join(&cache_key)
            } else {
                // Fall back to the directory next to the executable
                exe_dir().join("resources").join(&cache_key)
            }
        }
    }
}

fn resource_fingerprint() -> String {
    let mut files = super::load_resource_from_include_dir();
    files.sort_by(|left, right| left.path.cmp(&right.path));

    let mut hasher = Sha256::new();
    for file in files {
        hasher.update(file.path.as_bytes());
        hasher.update([0]);
        hasher.update(file.content.as_bytes());
        hasher.update([0]);
    }
    format!("{:x}", hasher.finalize())
}

fn exe_dir() -> PathBuf {
    let mut exe = std::env::current_exe().expect("executable available");
    exe.pop();
    exe
}
