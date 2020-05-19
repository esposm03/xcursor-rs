//! A crate to load cursor themes, and parse XCursor files.

use std::env::var;
use std::path::PathBuf;
use std::fs::File;
use std::io::Read;

use regex::Regex;

/// A module implementing XCursor file parsing.
pub mod parser;

/// This function returns the list of paths where the themes have to
/// be searched, according to the XDG Icon Theme specification.
///
/// # Panics
///
/// If the $HOME environment variable is not set,
/// or if its value contains the NUL character
pub fn theme_search_paths() -> Vec<PathBuf> {
    let mut res: Vec<PathBuf> = Vec::new();

    res.push([var("HOME").unwrap(), String::from(".icons")].iter().collect());

    for i in var("XDG_DATA_DIRS").unwrap_or("/usr/local/share/:/usr/share/".to_string()).split(':') {
        res.push([i, "icons"].iter().collect());
    }

    res.push(PathBuf::from("/usr/share/pixmaps"));

    res
}


/// A struct representing a cursor theme.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CursorTheme {
    name: String,
    dirs: Vec<PathBuf>,
    inherits: String,
    search_paths: Vec<PathBuf>,
}

impl CursorTheme {

    /// This function searches for a theme with the given name
    /// in the given search paths, and returns an XCursorTheme which
    /// represents it.
    /// If no inheritance can be determined, then the themes inherits
    /// from the "default" theme.
    pub fn load(name: &str, search_paths: &Vec<PathBuf>) -> Self {
        let mut dirs = Vec::new();
        let mut inherits = String::from("default");

        // Find dirs
        for mut path in search_paths.clone() {
            path.push(name);
            if path.is_dir() {
                dirs.push(path.clone());
            }
        }

        // Find inheritance
        for mut path in dirs.clone() {
            path.push("index.theme");

            if let Some(i) = theme_inherits(&path) {
                inherits = i;
            }
        }

        CursorTheme {
            name: String::from(name),
            dirs,
            inherits,
            search_paths: search_paths.clone(),
        }
    }

    /// Attempts to load an icon from the theme.
    /// If the icon is not found within this theme's
    /// directories, then the function looks at the
    /// theme from which this theme is inherited.
    pub fn load_icon(&self, icon_name: &str) -> Option<PathBuf> {
        for mut icon_path in self.dirs.clone() {
            icon_path.push("cursors");
            icon_path.push(icon_name);

            if icon_path.is_file() {
                return Some(icon_path);
            }
        }

        // If we're trying to find the inheritance of default
        if self.name == self.inherits {
            return None;
        }

        CursorTheme::load(&self.inherits, &self.search_paths).load_icon(icon_name)
    }

}

/// Loads the specified index.theme file, and returns a Some with
/// the value of the Inherits key in it.
/// Returns None if the file cannot be read for any reason,
/// if the file cannot be parsed, or if the `Inherits` key is omitted.
pub fn theme_inherits(file_path: &PathBuf) -> Option<String> {
    let mut content = String::new();
    let mut file = File::open(file_path).ok()?;
    file.read_to_string(&mut content).ok()?;

    let re = Regex::new(r"Inherits\s*=\s*([a-zA-Z0-9-_]+)").unwrap();
    let matches = re.captures_iter(&content).nth(0)?;

    Some(String::from(&matches[1]))
}

