use std::mem::take;

use color_eyre::eyre::{eyre, Result};
use futures::future::try_join_all;
use mime::Mime;
use serde::Deserialize;
use sled::Tree;
use tracing::{debug, info, trace, warn};

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

	let body = resp.text().await?;
	debug!(%url, chars=%body.chars().count(), "decoded body as text");
	trace!(%url, body=%body, "decoded body");

	let items = match (
		media_type.type_(),
		media_type.subtype().as_str(),
		media_type.suffix().map(|s| s.as_str()),
	) {
		(mime::APPLICATION, "atom", Some("xml")) => {
			parse_atom(&url, &body)?
		},
		(mime::APPLICATION, "rss", Some("xml")) => {
			parse_rss(&url, &body)?
		}
		(mime::APPLICATION, "xml", None) => {
			warn!(%url, ?media_type, "ambiguous media type, trying rss");
			parse_rss(&url, &body).or_else(|err| {
				warn!(%url, %err, "failed to parse as rss, trying atom");
				parse_atom(&url, &body)
			})?
		}
		_ => return Err(eyre!("unsupported media type: {}", media_type)),
	};

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

fn parse_rss(url: &str, body: &str) -> Result<Vec<Item>> {
	let mut rss: Rss = serde_xml_rs::from_str(body)?;
	trace!(%url, ?rss, "parsed rss");

	let items: Vec<Item> = take(&mut rss.channel.items).into_iter().map(Item::from).collect();
	info!(%url, "found {} items in rss", items.len());

	Ok(items)
}

fn parse_atom(url: &str, body: &str) -> Result<Vec<Item>> {
	let mut atom: Atom = serde_xml_rs::from_str(body)?;
	trace!(%url, ?atom, "parsed atom");

	let items: Vec<Item> = take(&mut atom.entries).into_iter().map(Item::from).collect();
	info!(%url, "found {} items in atom", items.len());

	Ok(items)
}

#[derive(Clone, Debug, Deserialize)]
struct Rss {
	pub channel: Channel,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename = "feed")]
struct Atom {
	#[serde(rename = "entry", default)]
	pub entries: Vec<AtomEntry>,
}

#[derive(Clone, Debug, Deserialize)]
struct Channel {
	#[serde(rename = "item", default)]
	pub items: Vec<RssItem>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Item {
	pub title: String,
	pub guid: String,
	pub link: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
struct RssItem {
	title: String,
	guid: String,
	link: String,
}

impl From<RssItem> for Item {
	fn from(item: RssItem) -> Self {
		Self { title: item.title, guid: item.guid, link: item.link }
	}
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
struct AtomEntry {
	title: String,
	id: String,
	#[serde(rename = "link")]
	link: AtomLink,
}

impl From<AtomEntry> for Item {
	fn from(entry: AtomEntry) -> Self {
		Self { title: entry.title, guid: entry.id, link: entry.link.href }
	}
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
struct AtomLink {
	href: String,
}
