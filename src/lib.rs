//! A crate to load cursor themes, and parse XCursor files.

use std::env;
use std::path::{Path, PathBuf};

/// A module implementing XCursor file parsing.
pub mod parser;

/// Get the list of paths where the themes have to be searched,
/// according to the XDG Icon Theme specification, respecting `XCURSOR_PATH` env
/// variable, in case it was set.
pub fn theme_search_paths() -> Vec<PathBuf> {
    // Handle the `XCURSOR_PATH` env variable, which takes over default search paths for cursor
    // theme. Some systems rely are using non standard directory layout and primary using this
    // env variable to perform cursor loading from a right places.
    let xcursor_path = match env::var("XCURSOR_PATH") {
        Ok(xcursor_path) => xcursor_path.split(':').map(PathBuf::from).collect(),
        Err(_) => {
            // Get icons locations from XDG data directories.
            let get_icon_dirs = |xdg_path: String| -> Vec<PathBuf> {
                xdg_path
                    .split(':')
                    .map(|entry| {
                        let mut entry = PathBuf::from(entry);
                        entry.push("icons");
                        entry
                    })
                    .collect()
            };

            let mut xdg_data_home = get_icon_dirs(
                env::var("XDG_DATA_HOME").unwrap_or_else(|_| String::from("~/.local/share")),
            );

            let mut xdg_data_dirs = get_icon_dirs(
                env::var("XDG_DATA_DIRS")
                    .unwrap_or_else(|_| String::from("/usr/local/share:/usr/share")),
            );

            let mut xcursor_path =
                Vec::with_capacity(xdg_data_dirs.len() + xdg_data_home.len() + 4);

            // The order is following other XCursor loading libs, like libwayland-cursor.
            xcursor_path.append(&mut xdg_data_home);
            xcursor_path.push(PathBuf::from("~/.icons"));
            xcursor_path.append(&mut xdg_data_dirs);
            xcursor_path.push(PathBuf::from("/usr/share/pixmaps"));
            xcursor_path.push(PathBuf::from("~/.cursors"));
            xcursor_path.push(PathBuf::from("/usr/share/cursors/xorg-x11"));

            xcursor_path
        }
    };

    let homedir = env::var("HOME");

    xcursor_path
        .into_iter()
        .filter_map(|dir| {
            // Replace `~` in a path with `$HOME` for compatibility with other libs.
            let mut expaned_dir = PathBuf::new();

            for component in dir.iter() {
                if component == "~" {
                    let homedir = match homedir.as_ref() {
                        Ok(homedir) => homedir.clone(),
                        Err(_) => return None,
                    };

                    expaned_dir.push(homedir);
                } else {
                    expaned_dir.push(component);
                }
            }

            Some(expaned_dir)
        })
        .collect()
}

/// A cursor theme.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CursorTheme {
    name: String,
    dirs: Vec<PathBuf>,
    inherits: String,
    search_paths: Vec<PathBuf>,
}

impl CursorTheme {
    /// Search for a theme with the given name in the given search paths,
    /// and returns an XCursorTheme which represents it. If no inheritance
    /// can be determined, then the themes inherits from the "default" theme.
    pub fn load(name: &str, search_paths: Vec<PathBuf>) -> Self {
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
            search_paths,
        }
    }

    /// Try to load an icon from the theme.
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

        CursorTheme::load(&self.inherits, self.search_paths.clone()).load_icon(icon_name)
    }
}

/// Load the specified index.theme file, and returns a `Some` with
/// the value of the `Inherits` key in it.
/// Returns `None` if the file cannot be read for any reason,
/// if the file cannot be parsed, or if the `Inherits` key is omitted.
pub fn theme_inherits(file_path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(file_path).ok()?;

    parse_theme(&content)
}

/// Parse the content of the `index.theme` and return the `Inherits` value.
fn parse_theme(content: &str) -> Option<String> {
    const PATTERN: &str = "Inherits";

    let is_xcursor_space_or_separator =
        |&ch: &char| -> bool { ch.is_whitespace() || ch == ';' || ch == ',' };

    for line in content.lines() {
        // Line should start with `Inherits`, otherwise go to the next line.
        if !line.starts_with(PATTERN) {
            continue;
        }

        // Skip the `Inherits` part and trim the leading white spaces.
        let mut chars = line.get(PATTERN.len()..).unwrap().trim_start().chars();

        // If the next character after leading white spaces isn't `=` go the next line.
        if Some('=') != chars.next() {
            continue;
        }

        // Skip XCursor spaces/separators.
        let result: String = chars
            .skip_while(is_xcursor_space_or_separator)
            .take_while(|ch| !is_xcursor_space_or_separator(ch))
            .collect();

        if !result.is_empty() {
            return Some(result);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::parse_theme;

    #[test]
    fn parse_inherits() {
        let theme_name = String::from("XCURSOR_RS");

        let theme = format!("Inherits={}", theme_name.clone());

        assert_eq!(parse_theme(&theme), Some(theme_name.clone()));

        let theme = format!(" Inherits={}", theme_name.clone());

        assert_eq!(parse_theme(&theme), None);

        let theme = format!(
            "[THEME name]\nInherits   = ,;\t\t{};;;;Tail\n\n",
            theme_name.clone()
        );

        assert_eq!(parse_theme(&theme), Some(theme_name.clone()));

        let theme = format!("Inherits;=;{}", theme_name.clone());

        assert_eq!(parse_theme(&theme), None);

        let theme = format!("Inherits = {}\n\nInherits=OtherTheme", theme_name.clone());

        assert_eq!(parse_theme(&theme), Some(theme_name.clone()));

        let theme = format!(
            "Inherits = ;;\nSome\tgarbage\nInherits={}",
            theme_name.clone()
        );

        assert_eq!(parse_theme(&theme), Some(theme_name.clone()));
    }
}
