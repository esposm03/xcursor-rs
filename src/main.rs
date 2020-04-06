use xcursor::theme_search_dirs;
use std::env::var;

fn main() {
    println!("{:?}", theme_search_dirs());
}
