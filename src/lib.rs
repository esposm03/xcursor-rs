use std::env::var;
use std::path::PathBuf;

/// This function returns the list of paths where the themes have to
/// be searched, according to the XDG Icon Theme specification.
///
/// # Panics
///
/// If the $HOME environment variable is not set,
/// or if its value contains the NUL character
pub fn theme_search_dirs() -> Vec<PathBuf> {
    let mut res: Vec<PathBuf> = Vec::new();

    res.push([var("HOME").unwrap(), String::from(".icons")].iter().collect());

    for i in var("XDG_DATA_DIRS").unwrap_or("/usr/local/share/:/usr/share/".to_string()).split(':') {
        res.push([i, "icons"].iter().collect());
    }

    res.push(PathBuf::from("/usr/share/pixmaps"));

    res
}

