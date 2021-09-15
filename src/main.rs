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

mod cap;
mod feed;
mod bound;

#[derive(Clone, Debug, StructOpt)]
struct Args {
	#[structopt(short, parse(from_occurrences))]
	verbose: u8,

	#[structopt(long, default_value = "https://alerts.metservice.com/cap/rss")]
	cap_rss: Vec<String>,

	#[structopt(long, default_value = ".")]
	boundaries: PathBuf,

	#[structopt(long, default_value = "_cache")]
	cache_db: PathBuf,

	#[structopt(long)]
	workplace_token: Option<String>,

	#[structopt(long)]
	workplace_group: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
	color_eyre::install()?;

	if var("RUST_LOG").is_ok() {
		tracing_subscriber::fmt::init();
	}

	let args = Args::from_args();

	tracing_subscriber::fmt()
		.with_env_filter(match args.verbose {
			0 => "capchat=info",
			1 => "capchat=debug",
			2 => "capchat=trace",
			3.. => "trace",
		})
		.try_init()
		.ok();

	debug!(?args, "parsed arguments");

	debug!(path=?args.cache_db, "opening sled database");
	let db = sled::open(args.cache_db)?;
	db.drop_tree("cache")?; // DEV
	let cache = db.open_tree("cache")?;

	let caps = try_join_all(args.cap_rss.iter().cloned().map(move |url| {
		let tree = cache.clone();
		tokio::spawn(async move { feed::fetch_feed(tree, url).await })
	}))
	.await?
	.into_iter()
	.collect::<Result<Vec<_>, _>>()?
	.into_iter()
	.flatten()
	.collect::<HashSet<_>>();

	debug!("fetched {} new caps", caps.len());

	// parse local geojson of areas we care about

	let mut bounds = Vec::with_capacity(1);
	for entry in glob::glob(args.boundaries.join("*.geojson").display().to_string().as_str())? {
		bounds.push(tokio::spawn(async { bound::read_geojson(entry?).await }));
	}

	let bounds = try_join_all(bounds).await?
		.into_iter()
		.collect::<Result<Vec<_>, _>>()?
		.into_iter()
		.flatten()
		.collect::<Vec<_>>();

	// filter for intersections and levels we care about
	// prepare for display
	// print out
	// make call to chat api (in prod)

	Ok(())
}
