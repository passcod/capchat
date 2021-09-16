use std::{collections::HashSet, str::FromStr};

use color_eyre::eyre::Result;
use itertools::Itertools;
use tracing::debug;

use crate::cap::Cap;

pub fn text(caps: HashSet<Cap>) -> Result<Out> {
	let mut message = String::with_capacity(caps.len() * 512);

	let headlines = caps
		.iter()
		.sorted_by_key(|c| &c.info.headline)
		.group_by(|c| &c.info.headline);

	for (headline, caps) in &headlines {
		message.extend(
			format!(
				"{}\n\n{}\n",
				headline.to_uppercase(),
				caps.map(|c| {
					format!(
						"{} [{}]  {} hours from {} to {}\n\n{}\n\n",
						c.info
							.parameters
							.get("ColourCode")
							.and_then(|c| colour_code_emoji(c.as_str()))
							.unwrap_or(' '),
						c.info.areas.iter().map(|a| &a.desc).join(", "),
						c.info
							.expires
							.signed_duration_since(c.info.onset)
							.num_hours(),
						c.info.onset.format("%_I:%M%P %A"),
						c.info.expires.format("%_I:%M%P %A"),
						c.info.description,
					)
					.trim_start()
					.to_string()
				})
				.join("\n\n")
			)
			.chars(),
		);
	}

	Ok(Out {
		message: message.trim().into(),
		image: None,
	})
}

pub fn image(_caps: HashSet<Cap>) -> Result<Out> {
	todo!()
}

pub fn image_with_map(_caps: HashSet<Cap>) -> Result<Out> {
	todo!()
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Out {
	pub message: String,
	pub image: Option<Vec<u8>>, // TODO
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
	Json,
	Text,
	Image,
	ImageMap,
}

impl FromStr for OutputFormat {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"json" => Ok(Self::Json),
			"text" => Ok(Self::Text),
			"image" => Ok(Self::Image),
			"map" | "image+map" => Ok(Self::ImageMap),
			_ => Err(format!("unknown output type: {}", s)),
		}
	}
}
