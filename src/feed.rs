use std::{
	collections::{HashMap, HashSet},
	env::var,
	hash::{Hash, Hasher},
	mem::take,
	num::ParseFloatError,
	path::PathBuf,
	str::FromStr,
};

use chrono::{DateTime, Utc};
use color_eyre::eyre::{eyre, Result};
use futures::future::try_join_all;
use geo::{Coordinate, CoordNum, LineString, Polygon};
use serde::{Deserialize, Deserializer};
use sled::Tree;
use structopt::StructOpt;
use tracing::{debug, error, info, trace};

use crate::cap::fetch_cap;

pub async fn fetch_feed(cache: Tree, url: String) -> Result<Vec<Cap>> {
	info!(%url, "fetching CAP feed");
	let resp = reqwest::get(&url).await?;

	if !resp.status().is_success() {
		error!(status=%resp.status(), "failed to fetch feed");
		return Err(eyre!("failed to fetch feed {}", url));
	}

	info!(
		%url,
		age=%resp.headers().get("age").map(|v| v.to_str()).transpose()?.unwrap_or("?"),
		bytes=%resp.headers().get("content-length").map(|v| v.to_str()).transpose()?.unwrap_or("?"),
		"got CAP feed"
	);
	trace!(status=?resp.status(), headers=?resp.headers(), "headers");

	let body = resp.text().await?;
	debug!(%url, chars=%body.len(), "decoded body as text");
	trace!(%url, body=%body, "decoded body");

	let mut rss: Rss = serde_xml_rs::from_str(&body)?;
	trace!(%url, ?rss, "parsed rss");

	let items = take(&mut rss.channel.items);
	info!(%url, ?rss, "found {} items in rss", items.len());

	let mut new = Vec::with_capacity(items.len());
	for item in items {
		trace!(%url, guid=%item.guid, "checking item against cache");

		if cache
			.compare_and_swap(
				item.guid.as_bytes().clone(),
				None::<Vec<u8>>,
				Some(item.link.as_bytes().clone()),
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
	let caps = try_join_all(new.into_iter().map(move |item|
		tokio::spawn(async move { fetch_cap(item).await })
	))
	.await?
	.into_iter()
	.collect::<Result<_, _>>()?;

	Ok(caps)
}

#[derive(Clone, Debug, Deserialize)]
struct Rss {
	pub channel: Channel,
}

#[derive(Clone, Debug, Deserialize)]
struct Channel {
	pub title: String,
	pub link: String,
	pub description: String,

	#[serde(rename = "pubDate")]
	pub pub_date: String,

	#[serde(rename = "item")]
	pub items: Vec<Item>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
struct Item {
	pub title: String,
	pub guid: String,
	pub category: String,
	pub description: String,
	pub link: String,

	#[serde(rename = "pubDate")]
	pub pub_date: String,
}
