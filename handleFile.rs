use std::{fs, path::PathBuf};
use walkdir::WalkDir;

pub fn getfiles(path_buffer: &mut Vec<PathBuf>) {
    let root_path = "/"//"/home/lichango/Programacion/rust/Roulette/testdir"; //TODO test dir to
    //not mess up
    WalkDir::new(root_path)
        .into_iter()
        .filter_entry(|e| {
            let path = e.path();
            //reading a virtual dir will max out memory or some other unpredictable (not fun) errors
            if path.starts_with("/proc") || path.starts_with("/sys") || path.starts_with("/dev") {
                return false;
            }
            true
        })
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .for_each(|entry| path_buffer.push(entry.path().to_path_buf()));
}

pub fn delete_file(to_delete: PathBuf) -> std::io::Result<()> {
    fs::remove_file(to_delete)?;
    Ok(())
}
