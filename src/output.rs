use std::str::FromStr;

use color_eyre::eyre::Result;
use itertools::Itertools;
use tracing::debug;

pub use map::text_with_map;
pub use text::text;

mod map;
mod text;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Out {
	pub message: String,
	pub image: Option<Vec<u8>>, // TODO
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
	Json,
	Text,
	Map,
}

impl FromStr for OutputFormat {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"json" => Ok(Self::Json),
			"text" => Ok(Self::Text),
			"map" | "text+map" => Ok(Self::Map),
			_ => Err(format!("unknown output type: {}", s)),
		}
	}
}

pub fn colour_code_emoji(c: &str) -> Option<char> {
	match c.to_lowercase().as_str() {
		"blue" => Some('🔵'),
		"green" => Some('🟢'),
		"yellow" => Some('🟡'),
		"orange" => Some('🟠'),
		"red" => Some('🔴'),
		"purple" => Some('🟣'),
		_ => None,
	}
}

pub fn split_long_message(out: Out, max_len: usize, min_len: usize) -> (Out, Option<Out>) {
	let len = out.message.chars().count();
	if len <= max_len {
		debug!(max=%max_len, %len, "remainder is below maximum, sending as is");
		return (out, None);
	}

	let max = if len - max_len < min_len {
		min_len
	} else {
		max_len
	};

	debug!(%max, "splitting message along whitespace");
	let mut text = out.message.split(' ').peekable();

	let mut n = 0;
	let first = text
		.peeking_take_while(|frag| {
			n += frag.chars().count() + 1;
			n < max
		})
		.join(" ");
	let rest = text.join(" ");

	debug!(
		max=%max_len,
		first=%first.chars().count(),
		rest=%rest.chars().count(),
		"split message into two",
	);

	let mut first_out = out.clone();
	first_out.message = first;

	(
		first_out,
		Some(Out {
			message: rest,
			..Out::default()
		}),
	)
}
