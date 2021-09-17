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

pub async fn send(webhook_url: &str, out: &Out) -> Result<()> {
	let mut out = if let Out {
		message,
		image: Some(img),
	} = out
	{
		let image_only = Out {
			message: "".into(),
			image: Some(img.clone()),
		};
		send_message(webhook_url, &image_only).await?;
		Out {
			message: message.clone(),
			image: None,
		}
	} else {
		out.clone()
	};

	loop {
		let (first, rest) = split_long_message(out, 2000, 280);
		send_message(webhook_url, &first).await?;
		out = match rest {
			Some(o) => o,
			None => break,
		};
	}

	Ok(())
}

async fn send_message(webhook_url: &str, out: &Out) -> Result<()> {
	let client = Client::new();
	let req = client.post(webhook_url);

	let req = if let Some(ref imagedata) = out.image {
		assert!(out.message.is_empty(), "cannot send both text and an image");

		let part = Part::bytes(imagedata.clone())
			.file_name("image.png")
			.mime_str("image/png")?;
		let form = Form::new().part("file", part);

		req.multipart(form)
	} else {
		req.header("Content-Type", "application/json")
			.json(&Payload {
				content: out.message.clone(),
				..Default::default()
			})
	};

	trace!(?req, "sending request");
	let resp = req.send().await?;
	trace!(?resp, "response from discord");
	let status = resp.status();

	let body: HashMap<String, Value> = resp.json().await?;
	trace!(?body, "response body");

	if !status.is_success() {
		Err(eyre!(
			"failed to send message to discord: {}\n{:?}",
			status,
			body
		))
	} else {
		Ok(())
	}
}

#[derive(Clone, Debug, Default, Serialize)]
struct Payload {
	#[serde(skip_serializing_if = "String::is_empty")]
	content: String,

	#[serde(skip_serializing_if = "String::is_empty")]
	username: String,

	#[serde(skip_serializing_if = "String::is_empty")]
	avatar_url: String,
}
