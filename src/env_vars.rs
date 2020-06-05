use std::env;

/// Substitute all the variables in the provided strings.
///
/// Given a slice of `str`s, all variables will be substituted with
/// their value. (Note that the syntax `${var_name_here}` is not supported)
/// This function will also handle PATH-like colon-separated segments in variable values.
///
/// Note that this function allocates quite a bit, but it is ideally only called once
/// (when a cursor theme is first loaded).
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

/// Helper function for `substitute_variables`, to split off logic.
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

/// Substitutes the specified variable in the given string.
///
/// If the variable can't be retrieved, then it isn't substituted.
///
/// If the variable contains multiple colon-separated segments
/// (akin to the unix PATH variable, or the XDG Base Directory ones),
/// then multiple strings will be generated, one for each segment.
fn substitute_single_variable(in_str: &str, name: &str) -> Vec<String> {
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
fn find_first_variable(input: &str) -> Option<&str> {
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

	fn test_common() {
		println!("Note: since the test uses environment variables, running multiple tests in parallel causes a race. Try to re-run the tests with `cargo test -- --test-threads 1`");
	}

	#[test]
	fn test_first_variable() {
		test_common();
		let string = "hello$VAR_1/world$VAR_2";
		assert_eq!(Some("VAR_1"), find_first_variable(string));
	}

	#[test]
	fn test_first_variable_at_end() {
		test_common();
		let string = "hello/world/$VAR_1";
		assert_eq!(Some("VAR_1"), find_first_variable(string));
	}

	#[test]
	fn test_first_variable_no_variable() {
		test_common();
		let string = "hello/world";
		assert_eq!(None, find_first_variable(string));
	}

	#[test]
	fn test_substitute_single_variable() {
		test_common();
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
		test_common();
		let string = "$XDG_CONFIG_HOME/xcursor-rs/";

		env::remove_var("XDG_CONFIG_HOME");

		assert_eq!(
			substitute_single_variable(string, "XDG_CONFIG_HOME")[0],
			"$XDG_CONFIG_HOME/xcursor-rs/"
		);
	}

	#[test]
	fn test_substitute_single_variable_multiple_vars() {
		test_common();
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
		test_common();
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
		test_common();
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
		test_common();
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
