use std::collections::HashMap;

use color_eyre::eyre::{eyre, Result};
use reqwest::{
	multipart::{Form, Part},
	Client,
};
use serde::Serialize;
use serde_json::Value;
use tracing::trace;

use crate::output::{split_long_message, Out};

pub async fn send(token: &str, thread_id: &str, out: &Out) -> Result<()> {
	let mut out = if let Out {
		message,
		image: Some(img),
	} = out
	{
		let image_only = Out {
			message: "".into(),
			image: Some(img.clone()),
		};
		send_message(token, thread_id, &image_only).await?;
		Out {
			message: message.clone(),
			image: None,
		}
	} else {
		out.clone()
	};

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
	let req = client
		.post("https://graph.facebook.com/me/messages")
		.bearer_auth(token);

	let req = if let Some(ref imagedata) = out.image {
		assert!(out.message.is_empty(), "cannot send both text and an image");

		let part = Part::bytes(imagedata.clone())
			.file_name("image.png")
			.mime_str("image/png")?;
		let form = Form::new()
			.text(
				"recipient",
				serde_json::to_string(&Recipient {
					thread_key: thread_id.into(),
				})?,
			)
			.text("message", serde_json::to_string(&Message::image())?)
			.part("filedata", part);

		req.multipart(form)
	} else {
		req.header("Content-Type", "application/json")
			.json(&MessageData {
				message: Message::text(&out.message),
				recipient: Recipient {
					thread_key: thread_id.into(),
				},
			})
	};

	trace!(?req, "sending request");
	let resp = req.send().await?;
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
	#[serde(skip_serializing_if = "String::is_empty")]
	text: String,

	#[serde(skip_serializing_if = "Option::is_none")]
	attachment: Option<Attachment>,
}

impl Message {
	fn text(text: impl Into<String>) -> Self {
		Self {
			text: text.into(),
			attachment: None,
		}
	}

	fn image() -> Self {
		Self {
			text: "".into(),
			attachment: Some(Attachment::default()),
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
