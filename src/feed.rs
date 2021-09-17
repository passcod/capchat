use color_eyre::eyre::Result;
use futures::future::try_join_all;
use itertools::Itertools;
use mime::Mime;

use sled::Tree;
use tracing::{debug, info, trace};

use crate::cap::{fetch_cap, Cap};

pub async fn fetch_feed(cache: Tree, url: String) -> Result<Vec<Cap>> {
	info!(%url, "fetching CAP feed");
	let resp = reqwest::get(&url).await?.error_for_status()?;

	let content_type = resp
		.headers()
		.get("content-type")
		.map(|v| v.to_str())
		.transpose()?
		.unwrap_or("?");

	info!(
		%url,
		age=%resp.headers().get("age").map(|v| v.to_str()).transpose()?.unwrap_or("?"),
		bytes=%resp.content_length().unwrap_or(0),
		%content_type,
		"got CAP feed"
	);
	trace!(status=?resp.status(), headers=?resp.headers(), "headers");

	let media_type: Mime = content_type.parse()?;
	trace!(%url, ?media_type, "decoded media type");

	let body = resp.bytes().await?;
	debug!(%url, bytes=%body.len(), "read body");
	trace!(%url, ?body, "full body");

	let feed = feed_rs::parser::parse(&*body)?;
	trace!(%url, ?feed, "parsed feed");
	let items = feed
		.entries
		.into_iter()
		.filter_map(|entry| {
			if entry.links.len() == 1 {
				Some(Item {
					guid: entry.id,
					link: entry.links.first().unwrap().href.clone(),
				})
			} else if let Some(link) = entry
				.links
				.into_iter()
				.find(|link| link.media_type == Some("application/cap+xml".to_string()))
			{
				Some(Item {
					guid: entry.id,
					link: link.href,
				})
			} else {
				None
			}
		})
		.collect_vec();
	debug!(%url, ?items, "extracted {} items", items.len());

	let mut new = Vec::with_capacity(items.len());
	for item in items {
		trace!(%url, guid=%item.guid, "checking item against cache");

		if cache
			.compare_and_swap(
				item.guid.as_bytes().to_owned(),
				None::<Vec<u8>>,
				Some(item.link.as_bytes().to_owned()),
			)?
			.is_err()
		{
			trace!(%url, guid=%item.guid, "item already in cache, skipping");
		} else {
			trace!(%url, guid=%item.guid, "item wasn't in cache, keeping");
			new.push(item);
		}
	}

	info!(%url, "after cache pass, remains {} items", new.len());
	trace!(%url, ?new, "new items");

	debug!(%url, "fetching CAPs for new items");
	let caps = try_join_all(
		new.into_iter()
			.map(move |item| tokio::spawn(async move { fetch_cap(item).await })),
	)
	.await?
	.into_iter()
	.collect::<Result<_, _>>()?;

	Ok(caps)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
	pub guid: String,
	pub link: String,
}
