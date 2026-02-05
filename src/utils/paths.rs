use directories::ProjectDirs;
use std::path::PathBuf;
use std::fs;

pub fn get_data_dir() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("com", "picaclip", "picaclip") {
        let data_dir = proj_dirs.data_dir();
        if !data_dir.exists() {
            fs::create_dir_all(data_dir).expect("Failed to create data directory");
        }
        return data_dir.to_path_buf();
    }
    PathBuf::from(".picaclip")
}

pub fn get_db_path() -> PathBuf {
    get_data_dir().join("data.db")
}

pub fn get_image_cache_dir() -> PathBuf {
    let dir = get_data_dir().join("images");
    if !dir.exists() {
        fs::create_dir_all(&dir).expect("Failed to create image cache directory");
    }
    dir
}
