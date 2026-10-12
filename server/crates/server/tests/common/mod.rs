//! A throwaway web root with a secret beside it, removed when the test ends.
#![allow(dead_code)]
use std::path::PathBuf;

pub const INDEX: &str = "<!doctype html><title>Sidereal</title>";
pub const SCRIPT: &str = "console.log('shell');";
pub const FOLDER_INDEX: &str = "<!doctype html><title>folder index</title>";
pub const SECRET: &str = "distinctive-secret-outside-the-web-root";

pub struct WebRoot {
    parent: PathBuf,
    pub path: PathBuf,
}

impl WebRoot {
    /// `name` must be unique among the tests of one binary.
    pub fn new(name: &str) -> Self {
        let parent = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("web-root-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&parent);
        let path = parent.join("web");
        std::fs::create_dir_all(path.join("assets")).unwrap();
        std::fs::write(parent.join("secret.txt"), SECRET).unwrap();
        std::fs::write(path.join("index.html"), INDEX).unwrap();
        std::fs::write(path.join("assets/app.js"), SCRIPT).unwrap();
        std::fs::write(path.join("assets/index.html"), FOLDER_INDEX).unwrap();
        std::fs::write(path.join("healthz"), "shadow").unwrap();
        std::fs::write(path.join("readyz"), "shadow").unwrap();
        Self { parent, path }
    }
}

impl Drop for WebRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.parent);
    }
}
