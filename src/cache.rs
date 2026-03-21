use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

const TTL: u16 = 60 * 60 * 12;

#[derive(Clone)]
pub struct Cache {
    path: PathBuf,
}

impl Cache {
    pub fn new(path: String) -> Self {
        let _ = fs::create_dir_all(&path);
        let path = Path::new(&path).to_owned();

        Cache { path }
    }

    pub fn put(&self, key: &str, binary: &[u8]) {
        let _ = fs::write(self.path.join(&format!("{}.bin", key)), binary);
    }

    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        let path = self.path.join(&format!("{}.bin", key,));
        if !fs::exists(&path).expect("check path") {
            return None;
        }

        let crtime = fs::metadata(&path)
            .expect("path metadata")
            .created()
            .expect("path crtime");

        if SystemTime::now() >= crtime.checked_add(Duration::from_secs(TTL as u64))? {
            fs::remove_file(path).expect("delete path");
            return None;
        };

        Some(fs::read(path).expect("read path"))
    }

    pub fn cleanup(&self) {
        let files = fs::read_dir(&self.path).expect("read dir");
        for entry in files {
            let entry = entry.expect("entry perms");
            let crtime = entry
                .metadata()
                .expect("entry metadata")
                .created()
                .expect("entry crtime");

            if SystemTime::now()
                >= crtime
                    .checked_add(Duration::from_secs(TTL as u64))
                    .expect("crtime oob")
            {
                fs::remove_file(entry.path()).expect("delete entry");
            }
        }
    }
}
