use std::collections::HashSet;

use color_eyre::eyre::Result;
use itertools::Itertools;

use crate::cap::Cap;
use super::{Out, colour_code_emoji};

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
