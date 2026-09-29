 
use walkdir::WalkDir;
use std::fs;

fn getfiles(pathBuffer: &mut Vec<PathBuf>){
    let root_path = "/";
    
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
        .for_each(|entry| {
            pathBuffer.push(entry.path().to_path_buf())
        });
}

fn chooseFile(){

}

fn deleteFile(toDelete: PathBuf) -> std::io::Result<()> {
    fs::remove_file(toDelete)?;
    Ok(())
}
