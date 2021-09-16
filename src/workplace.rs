use color_eyre::eyre::Result;
use reqwest::Client;
use serde::Serialize;

use crate::output::Out;

pub async fn send(token: &str, thread_id: &str, out: &Out) -> Result<()> {
	let client = Client::new();
	client.post("https://graph.facebook.com/me/messages")
		.bearer_auth(token)
		.json(&MessageData {
			message: out.message.to_string(),
			recipient: Recipient {
				thread_key: thread_id.into()
			},
		})
		.send()
		.await?
		.error_for_status()?;

	Ok(())
}

#[derive(Clone, Debug, Serialize)]
struct MessageData {
	message: String,
	recipient: Recipient,
}

#[derive(Clone, Debug, Serialize)]
struct Recipient {
	thread_key: String,
}
