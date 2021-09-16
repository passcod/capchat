use std::{collections::HashSet, str::FromStr};

use color_eyre::eyre::Result;
use itertools::Itertools;

use crate::cap::Cap;

pub fn text(caps: HashSet<Cap>) -> Result<Out> {
	let mut message = String::with_capacity(caps.len() * 512);

	let headlines = caps
		.iter()
		.sorted_by_key(|c| &c.info.headline)
		.group_by(|c| &c.info.headline);

	for (headline, caps) in &headlines {
		message.extend(format!(
			"{}\n\n{}\n",
			headline.to_uppercase(),
			caps.map(|c| {
				format!(
					"{} [{}]  {} hours from {} to {}\n\n{}\n\n",
					c.info.parameters.get("ColourCode").and_then(|c| colour_code_emoji(c.as_str())).unwrap_or(' '),
					c.info.areas.iter().map(|a| &a.desc).join(", "),
					c.info.expires.signed_duration_since(c.info.onset).num_hours(),
					c.info.onset.format("%_I:%M%P %A"),
					c.info.expires.format("%_I:%M%P %A"),
					c.info.description,
				).trim_start().to_string()
			}).join("\n\n")
		).chars());
	}

	Ok(Out { message: message.trim().into(), image: None })
}

pub fn image(_caps: HashSet<Cap>) -> Result<Out> {
	todo!()
}

pub fn image_with_map(_caps: HashSet<Cap>) -> Result<Out> {
	todo!()
}

fn colour_code_emoji(c: &str) -> Option<char> {
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Out {
	pub message: String,
	pub image: Option<()>, // TODO
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Output {
	Json,
	Text,
	Image,
	ImageMap,
}

impl FromStr for Output {
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
