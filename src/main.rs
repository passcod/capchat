use std::{collections::HashSet, env::var, path::PathBuf};

use color_eyre::eyre::Result;
use futures::future::try_join_all;
use geo::prelude::Intersects;
use structopt::StructOpt;
use tracing::{debug, info};

use output::Output;

mod bound;
mod cap;
mod feed;
mod output;

#[derive(Clone, Debug, StructOpt)]
struct Args {
	#[structopt(short, parse(from_occurrences))]
	verbose: u8,

	#[structopt(short, long)]
	quiet: bool,

	#[structopt(long)]
	cap: Vec<String>,

	#[structopt(long, default_value = "Minor")]
	severity: cap::Severity,

	#[structopt(long, default_value = ".")]
	boundaries: PathBuf,

	#[structopt(long, default_value = "_cache")]
	cache_db: PathBuf,

	#[structopt(long, default_value = "map")]
	output: Output,

	#[structopt(long)]
	workplace_token: Option<String>,

	#[structopt(long)]
	workplace_group: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
	color_eyre::install()?;

	if var("RUST_LOG").is_ok() {
		tracing_subscriber::fmt()
			.with_writer(std::io::stderr)
			.init();
	}

	let args = Args::from_args();

	if !args.quiet {
		tracing_subscriber::fmt()
			.with_writer(std::io::stderr)
			.with_env_filter(match args.verbose {
				0 => "capchat=info",
				1 => "capchat=debug",
				2 => "capchat=trace",
				3.. => "trace",
			})
			.try_init()
			.ok();
	}

	debug!(?args, "parsed arguments");

	debug!(path=?args.cache_db, "opening sled database");
	let db = sled::open(&args.cache_db)?;
	db.drop_tree("cache")?; // DEV
	let cache = db.open_tree("cache")?;

	let mut caps = try_join_all(args.cap.iter().cloned().map(move |url| {
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

	if caps.is_empty() {
		info!("nothing to do");
		return Ok(());
	}

	info!("loading geojson boundaries");

	let mut bounds = Vec::with_capacity(1);
	for entry in glob::glob(
		args.boundaries
			.join("*.geojson")
			.display()
			.to_string()
			.as_str(),
	)? {
		bounds.push(tokio::spawn(async { bound::read_geojson(entry?).await }));
	}

	let bounds = try_join_all(bounds)
		.await?
		.into_iter()
		.collect::<Result<Vec<_>, _>>()?
		.into_iter()
		.flatten()
		.collect::<Vec<_>>();

	info!(boundaries=%bounds.len(), "checking intersections");
	caps.retain(|cap| {
		cap.info
			.areas
			.iter()
			.map(|a| &a.polygons)
			.flatten()
			.any(|p| bounds.iter().any(|b| b.intersects(p)))
	});
	info!(caps=%caps.len(), "filtered caps against boundaries");

	caps.retain(|cap| cap.info.severity >= args.severity);
	info!(caps=%caps.len(), severity=?args.severity, "filtered caps against severity");

	let _output = match args.output {
		Output::Json => {
			serde_json::to_writer(std::io::stdout(), &caps)?;
			return Ok(());
		}
		Output::Text => {
			let out = output::text(caps)?;
			println!("{}", out.message);
			out
		},
		Output::Image => output::image(caps)?,
		Output::ImageMap => output::image_with_map(caps)?,
	};

	// make call to chat apis

	Ok(())
}
