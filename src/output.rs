use std::{collections::HashSet, str::FromStr};

use color_eyre::eyre::Result;

use crate::cap::Cap;

pub fn text(_caps: HashSet<Cap>) -> Result<Out> {
	todo!()
}

pub fn image(_caps: HashSet<Cap>) -> Result<Out> {
	todo!()
}

pub fn image_with_map(_caps: HashSet<Cap>) -> Result<Out> {
	todo!()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Out;

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
