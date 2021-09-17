use std::{collections::HashSet, env::var, path::PathBuf};

use color_eyre::eyre::Result;
use futures::future::try_join_all;
use geo::prelude::{Contains, Intersects};
use structopt::StructOpt;
use tokio::{fs::File, io::AsyncWriteExt};
use tracing::{debug, info};

use output::OutputFormat;

use crate::output::Out;

mod cap;
mod discord;
mod facebook;
mod feed;
mod geodirs;
mod output;

#[derive(Clone, Debug, StructOpt)]
pub struct Args {
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

	/// Type of output to send to chatrooms (`json`, `text`, `text+map`).
	#[structopt(long, default_value = "text+map")]
	format: OutputFormat,

	/// Print text output to STDOUT.
	#[structopt(long)]
	print: bool,

	/// Write output to file.
	///
	/// The message will go to `PATH.txt`, and if there's an image it will go to `PATH.png`.
	#[structopt(long)]
	file: Option<PathBuf>,

	/// Maximum height of image in pixels for `map` output format.
	#[structopt(long, default_value = "512")]
	image_height: u32,

	/// Maximum width of image in pixels for `map` output format.
	#[structopt(long, default_value = "512")]
	image_width: u32,

	/// Facebook Workplace token.
	///
	/// It must have _Message Any Member_ and _Group Chat Bot_ permissions.
	#[structopt(long)]
	facebook_token: Option<String>,

	/// Facebook Workplace Thread ID to post in.
	///
	/// This cannot be a single user chat, and the bot must already be in the group/thread.
	#[structopt(long)]
	facebook_thread: Option<String>,

	/// Discord webhook URL to use to post messages.
	#[structopt(long)]
	discord_webhook_url: Option<String>,
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
	// db.drop_tree("cache")?; // DEV
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
	let bounds = geodirs::load_polygons(&args.boundaries).await?;
	if !bounds.0.is_empty() {
		info!(boundaries=%bounds.0.len(), "checking intersections");
		caps.retain(|cap| {
			cap.info
				.areas
				.iter()
				.map(|a| &a.polygons)
				.flatten()
				.any(|p| bounds.intersects(p) || bounds.contains(p))
		});
		info!(caps=%caps.len(), "filtered caps against boundaries");
	}

	caps.retain(|cap| cap.info.severity >= args.severity);
	info!(caps=%caps.len(), severity=?args.severity, "filtered caps against severity");

	if caps.is_empty() {
		info!("nothing to do");
		return Ok(());
	}

	info!("formatting for output");
	let out = match args.format {
		OutputFormat::Json => Out {
			message: serde_json::to_string(&caps)?,
			..Out::default()
		},
		OutputFormat::Text => output::text(caps)?,
		OutputFormat::Map => output::text_with_map(caps, &args).await?,
	};

	if args.print {
		println!("{}", &out.message);
	}

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

	if let (Some(token), Some(thread)) = (&args.facebook_token, &args.facebook_thread) {
		info!(%thread, "sending to workplace");
		facebook::send(token, thread, &out).await?;
		debug!(%thread, "sent to workplace");
	}

	if let Some(webhook_url) = &args.discord_webhook_url {
		info!("sending to discord");
		discord::send(webhook_url, &out).await?;
		debug!("sent to discord");
	}

	info!("all done");
	Ok(())
}
