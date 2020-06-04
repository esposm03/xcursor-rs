use std::env;

type Range = std::ops::Range<usize>;

pub fn substitute_variable(in_str: &str, name: &str) -> Vec<String> {
	let mut vec = Vec::new();

	for current_segment in env::var(name).unwrap_or(String::new()).split(':') {
		let context = |actual: &str| {
			match name == actual {
				true => Some(current_segment),
				false => None,
			}
		};

		let substituted = String::from(shellexpand::full_with_context_no_errors(in_str, || env::var("HOME").ok(), context));
		vec.push(substituted);
	}

	vec
}

/// Find the index of the first variable in `input`, return its position and name.
pub fn find_next_variable_index(input: &str) -> Option<(Range, &str)> {
	let start = input.find('$')?;
	let trimmed: &str = &input[start..];
	let end = trimmed
		.find(|ch: char| ch != '_' && !ch.is_alphanumeric())
		.unwrap_or(trimmed.len() + start);

	Some((start..end - 1, &trimmed[1..]))
}

#[cfg(test)]
mod tests {

	#[test]
	fn rand_test() {
		std::env::set_var("XDG_DATA_DIRS", "/usr/share:/usr/local/share");
		std::env::set_var("XDG_DATA_DIR", "/hal:/vol");
		let mut vec = vec!["world$XDG_DATA_DIRS/pwoe$XDG_DATA_DIR".to_owned(), "hello$XDG_DATA_DIRS-adw/pwoe$XDG_DATA_DIR".to_owned()];
		super::substitute_variable(&mut vec, "XDG_DATA_DIRS");
		panic!();
	}
}
