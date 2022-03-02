use std::path::PathBuf;

pub struct FileSystem<'a> {
    base_path: &'a str,
}

impl<'a> FileSystem<'a> {
    pub fn new(base_path: &'a str) -> Self {
        FileSystem { base_path }
    }

    pub fn get_bundle_url() -> &'static str {
        ""
    }

    pub fn get_texture_file_path(&self, name: &str) -> PathBuf {
        PathBuf::from(self.base_path).join("assets").join(name)
    }
}
