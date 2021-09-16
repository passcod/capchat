use std::path::Path;

use color_eyre::eyre::Result;
use geo::Polygon;

use tracing::{debug, trace};

use crate::geodirs::{load_geo_dir, only_polys};

pub async fn load(path: impl AsRef<Path>) -> Result<Vec<Polygon<f64>>> {
	let path = path.as_ref();
	let gc = load_geo_dir(&path).await?;
	debug!(geos=%gc.0.len(), "loaded boundary geometries");

	let polys = only_polys(gc);
	trace!(?path, ?polys, "filtered to just polygons");
	debug!(?path, "obtained {} polygons", polys.len());

	Ok(polys)
}
