use std::env;

pub fn substitute_variables(strings: &[&str]) -> Vec<String> {
	let owned_strings = strings.iter().map(|el| String::from(*el)).collect();
	let mut vec = substitute_variables_pass(&owned_strings);

	loop {
		let old_vec = vec.clone();
		vec = substitute_variables_pass(&old_vec);
		if old_vec == vec {
			break;
		}
	}

	vec
}

fn substitute_variables_pass(strings: &Vec<String>) -> Vec<String> {
	let mut vec: Vec<String> = Vec::with_capacity(strings.len());

	for i in strings {
		match find_first_variable(i) {
			None => vec.push(i.to_string()),
			Some(var) => vec.extend_from_slice(&substitute_single_variable(i, var)),
		}
	}

	vec
}

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
	use super::{find_first_variable, substitute_single_variable, substitute_variables};
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

		env::set_var("HOME", "/home/alice");
		env::set_var("XDG_CONFIG_HOME", "~/.config");

		assert_eq!(
			substitute_single_variable(string, "XDG_CONFIG_HOME")[0],
			"/home/alice/.config/hello/$WORLD"
		);
	}

	#[test]
	fn test_substitute_variables_no_multiple_segments() {
		let string = "$XDG_CONFIG_HOME/hello/$XDG_DATA_HOME";

		env::set_var("HOME", "/home/alice");
		env::set_var("XDG_CONFIG_HOME", "~/.config");
		env::set_var("XDG_DATA_HOME", ".local/share");

		assert_eq!(
			substitute_variables(&[string])[0],
			"/home/alice/.config/hello/.local/share",
		)
	}

	#[test]
	fn test_substitute_variables_multiple_segments() {
		let string = "$XDG_DATA_DIRS/hello$XDG_CONFIG_DIRS";

		env::set_var("XDG_CONFIG_DIRS", "/etc/xdg:/etc/more_config");
		env::set_var("XDG_DATA_DIRS", "/usr/local/share:/usr/share");

		let mut expected = Vec::from(
			&[
				"/usr/local/share/hello/etc/xdg",
				"/usr/share/hello/etc/xdg",
				"/usr/share/hello/etc/more_config",
				"/usr/local/share/hello/etc/more_config",
			][..],
		);
		let mut got = substitute_variables(&[string]);

		// We don't care about ordering here.
		expected.sort();
		got.sort();

		assert_eq!(expected, got)
	}

	#[test]
	fn test_substitute_variables_multiple_strings() {
		let strings = ["$XDG_DATA_DIRS/hello/", "$XDG_CONFIG_DIRS/hello/"];

		env::set_var("XDG_CONFIG_DIRS", "/etc/xdg:/etc/more_config");
		env::set_var("XDG_DATA_DIRS", "/usr/local/share:/usr/share");

		let mut expected = Vec::from(
			&[
				"/usr/local/share/hello/",
				"/usr/share/hello/",
				"/etc/xdg/hello/",
				"/etc/more_config/hello/",
			][..],
		);
		let mut got = substitute_variables(&strings);

		expected.sort();
		got.sort();

		assert_eq!(expected, got);
	}
}
