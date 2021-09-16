use std::collections::HashSet;

use color_eyre::eyre::Result;

use super::{Out, text};
use crate::cap::Cap;
use crate::Args;

pub fn text_with_map(caps: HashSet<Cap>, _args: &Args) -> Result<Out> {
	let out = text(caps.clone())?;

	Ok(out)
}
