use std::{collections::HashSet, env::var, path::PathBuf};

use color_eyre::eyre::Result;
use futures::future::try_join_all;
use geo::prelude::Intersects;
use structopt::StructOpt;
use tokio::{fs::File, io::AsyncWriteExt};
use tracing::{debug, info};

use output::OutputFormat;

mod bound;
mod cap;
mod feed;
mod output;
mod workplace;

#[derive(Clone, Debug, StructOpt)]
struct Args {
	/// Increase logging verbosity (up to `-vvv`).
	///
	/// - `-v`: debug logs
	/// - `-vv`: trace logs just for this crate
	/// - `-vvv`: trace logs for everything (extremely chatty!)
	#[structopt(short, parse(from_occurrences))]
	verbose: u8,

	/// Suppress all logging output.
	#[structopt(short, long)]
	quiet: bool,

	/// URL(s) for the Atom/RSS feed to CAP alerts.
	#[structopt(long)]
	cap: Vec<String>,

	/// Minimum severity to get alerts for.
	#[structopt(long, default_value = "Minor")]
	severity: cap::Severity,

	/// Path to a folder container GeoJSON files with polygons that demarcate areas you care about.
	#[structopt(long, default_value = "_boundaries")]
	boundaries: PathBuf,

	/// Path to a folder container GeoJSON files with polygons for outlines of countries or areas, to render basemaps.
	#[structopt(long, default_value = "_outlines")]
	outlines: PathBuf,

	/// Path to the cache database (used to avoid double-posting).
	#[structopt(long, default_value = "_cache")]
	cache_db: PathBuf,

	/// Type of output to send to chatrooms (`json`, `text`, `image`, `map`).
	///
	/// If `json`, options related to output are ignored and the JSON is only printed to STDOUT.
	#[structopt(long, default_value = "map")]
	format: OutputFormat,

	/// Height of image in pixels for `image` and `map` output formats.
	#[structopt(long, default_value = "300")]
	image_height: u64,

	/// Width of image in pixels for `image` and `map` output formats.
	#[structopt(long, default_value = "400")]
	image_width: u64,

	/// Write output to file.
	///
	/// The message will go to `PATH.txt`, and if there's an image it will go to `PATH.png`.
	#[structopt(long)]
	file: Option<PathBuf>,

	/// Facebook Workplace token.
	///
	/// It must have _Message Any Member_ and _Group Chat Bot_ permissions.
	#[structopt(long)]
	fb_workplace_token: Option<String>,

	/// Facebook Workplace Thread ID to post in.
	///
	/// This cannot be a single user chat, and the bot must already be in the group/thread.
	#[structopt(long)]
	fb_workplace_thread: Option<String>,
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

	info!("formatting for output");
	let out = match args.format {
		OutputFormat::Json => {
			serde_json::to_writer(std::io::stdout(), &caps)?;
			return Ok(());
		}
		OutputFormat::Text => {
			let out = output::text(caps)?;
			println!("{}", &out.message);
			out
		}
		OutputFormat::Image => output::image(caps, args.image_width, args.image_height)?,
		OutputFormat::ImageMap => output::image_with_map(caps, args.image_width, args.image_height)?,
	};

	if let Some(path) = args.file {
		let mut txt = path.clone();
		txt.set_extension("txt");
		info!(path=?txt, "writing output message");
		File::create(txt)
			.await?
			.write_all(out.message.as_bytes())
			.await?;

		if let Some(ref bytes) = out.image {
			let mut img = path.clone();
			img.set_extension("png");
			info!(path=?img, "writing output image");
			File::create(img).await?.write_all(bytes).await?;
		}
	}

	if let (Some(token), Some(thread)) = (&args.fb_workplace_token, &args.fb_workplace_thread) {
		info!(%thread, "sending to workplace");
		workplace::send(token, thread, &out).await?;
		debug!(%thread, "sent to workplace");
	}

	info!("all done");
	Ok(())
}
