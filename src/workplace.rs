use std::collections::HashMap;

use color_eyre::eyre::{eyre, Result};
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use tracing::trace;

use crate::output::{split_long_message, Out};

pub async fn send(token: &str, thread_id: &str, out: &Out) -> Result<()> {
	let mut out = out.clone();
	loop {
		let (first, rest) = split_long_message(out, 2000, 280);
		send_message(token, thread_id, &first).await?;
		out = match rest {
			Some(o) => o,
			None => break,
		};
	}

	Ok(())
}

async fn send_message(token: &str, thread_id: &str, out: &Out) -> Result<()> {
	let client = Client::new();
	let resp = client
		.post("https://graph.facebook.com/me/messages")
		.bearer_auth(token)
		.header("Content-Type", "application/json")
		.json(&MessageData {
			message: Message::text(&out.message),
			recipient: Recipient {
				thread_key: thread_id.into(),
			},
		}) // TODO: image
		.send()
		.await?;
	trace!(?resp, "response from facebook");
	let status = resp.status();

	let body: HashMap<String, Value> = resp.json().await?;
	trace!(?body, "response body");

	if !status.is_success() {
		Err(eyre!(
			"failed to send message to facebook: {}\n{:?}",
			status,
			body
		))
	} else {
		Ok(())
	}
}

#[derive(Clone, Debug, Serialize)]
struct MessageData {
	message: Message,
	recipient: Recipient,
}

#[derive(Clone, Debug, Serialize)]
struct Recipient {
	thread_key: String,
}

#[derive(Clone, Debug, Default, Serialize)]
struct Message {
	text: String,

	#[serde(skip_serializing_if = "Option::is_none")]
	attachment: Option<Attachment>,

	#[serde(skip_serializing_if = "Option::is_none")]
	filedata: Option<String>,
}

impl Message {
	fn text(text: impl Into<String>) -> Self {
		Self {
			text: text.into(),
			..Self::default()
		}
	}

	fn text_and_image(text: impl Into<String>, image_ref: impl Into<String>) -> Self {
		Self {
			text: text.into(),
			attachment: Some(Attachment::default()),
			filedata: Some(image_ref.into()),
		}
	}
}

#[derive(Clone, Debug, Default, Serialize)]
struct Attachment {
	r#type: AttachmentType,
	payload: Payload,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
enum AttachmentType {
	Image,
}

impl Default for AttachmentType {
	fn default() -> Self {
		Self::Image
	}
}

#[derive(Clone, Debug, Default, Serialize)]
struct Payload {
	is_reusable: bool,
}
