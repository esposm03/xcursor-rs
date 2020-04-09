use xcursor::{theme_search_paths, XCursorTheme};

fn main() {
    let theme = XCursorTheme::load("breeze_cursors", &theme_search_paths());
    println!("{:#?}", theme);
}
