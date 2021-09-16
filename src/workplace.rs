use std::collections::HashMap;

use color_eyre::eyre::{eyre, Result};
use itertools::Itertools;
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use tracing::{debug, trace};

use crate::output::{Out, split_long_message};

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
	let resp = client.post("https://graph.facebook.com/me/messages")
		.bearer_auth(token)
		.header("Content-Type", "application/json")
		.json(&MessageData {
			message: Payload {
				text: out.message.to_string(),
			},
			recipient: Recipient {
				thread_key: thread_id.into()
			},
		})
		.send()
		.await?;
	trace!(?resp, "response from facebook");
	let status = resp.status();

	let body: HashMap<String, Value> = resp.json().await?;
	trace!(?body, "response body");

	if !status.is_success() {
		Err(eyre!("failed to send message to facebook: {}\n{:?}", status, body))
	} else {
		Ok(())
	}
}

#[derive(Clone, Debug, Serialize)]
struct MessageData {
	message: Payload,
	recipient: Recipient,
}

#[derive(Clone, Debug, Serialize)]
struct Payload {
	text: String,
}

#[derive(Clone, Debug, Serialize)]
struct Recipient {
	thread_key: String,
}
