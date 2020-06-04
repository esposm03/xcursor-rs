use std::env;

type Range = std::ops::Range<usize>;

pub fn substitute_single_variable(in_str: &str, name: &str) -> Vec<String> {
	let mut vec = Vec::new();

	let var = env::var(name);
	if var.is_err() {
		vec.push(String::from(in_str));
		return vec;
	}

	for current_segment in var.unwrap().split(':') {
		let context = |actual: &str| {
			if name == actual {
				Some(current_segment)
			} else {
				None
			}
		};

		// Here, we expand env and tilde separately because otherwise a tilde
		// inside a variable wouldn't be substituted (as of shellexpand v2.0).
		let substituted = shellexpand::env_with_context_no_errors(in_str, context);
		vec.push(String::from(shellexpand::tilde(&substituted)));
	}

	vec
}

/// Find the first variable in `input`, and return its name.
pub fn find_first_variable(input: &str) -> Option<&str> {
	let start = input.find('$')?;
	let trimmed: &str = &input[start + 1..];
	let end = trimmed
		.find(|ch: char| ch != '_' && !ch.is_alphanumeric())
		.unwrap_or(trimmed.len());

	Some(&trimmed[0..end])
}

#[cfg(test)]
mod tests {
	use super::{find_first_variable, substitute_single_variable};
	use std::env;

	#[test]
	fn test_first_variable() {
		let string = "hello$VAR_1/world$VAR_2";
		assert_eq!(Some("VAR_1"), find_first_variable(string));
	}

	#[test]
	fn test_first_variable_at_end() {
		let string = "hello/world/$VAR_1";
		assert_eq!(Some("VAR_1"), find_first_variable(string));
	}

	#[test]
	fn test_first_variable_no_variable() {
		let string = "hello/world";
		assert_eq!(None, find_first_variable(string));
	}

	#[test]
	fn test_substitute_single_variable() {
		let string = "$XDG_CONFIG_HOME/xcursor-rs/";

		env::set_var("HOME", "/home/alice");
		env::set_var("XDG_CONFIG_HOME", "~/.config");

		assert_eq!(
			substitute_single_variable(string, "XDG_CONFIG_HOME")[0],
			"/home/alice/.config/xcursor-rs/"
		);
	}

	#[test]
	fn test_substitute_single_variable_not_set() {
		let string = "$XDG_CONFIG_HOME/xcursor-rs/";
		env::remove_var("XDG_CONFIG_HOME");

		assert_eq!(
			substitute_single_variable(string, "XDG_CONFIG_HOME")[0],
			"$XDG_CONFIG_HOME/xcursor-rs/"
		);
	}

	#[test]
	fn test_substitute_single_variable_multiple_vars() {
		let string = "$XDG_CONFIG_HOME/hello/$WORLD";

		assert_eq!(
			substitute_single_variable(string, "XDG_CONFIG_HOME")[0],
			"/home/alice/.config/hello/$WORLD"
		);
	}
}
