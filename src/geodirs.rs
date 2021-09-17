use std::{
	convert::TryFrom,
	path::{Path, PathBuf},
};

use color_eyre::eyre::Result;
use futures::future::try_join_all;
use geo::{Geometry, GeometryCollection, MultiPolygon, Polygon};
use geojson::{quick_collection, FeatureCollection, GeoJson};
use tokio::{fs::File, io::AsyncReadExt};
use tracing::{debug, trace, warn};

pub async fn load_polygons(path: impl AsRef<Path>) -> Result<MultiPolygon<f64>> {
	let path = path.as_ref();
	let gc = load_geo_dir(&path).await?;
	debug!(geos=%gc.0.len(), "loaded boundary geometries");

	let polys = only_polys(gc);
	trace!(?path, ?polys, "filtered to just polygons");
	debug!(?path, "obtained {} polygons", polys.len());

	Ok(MultiPolygon(polys))
}

pub async fn load_geo_dir(path: impl AsRef<Path>) -> Result<GeometryCollection<f64>> {
	let mut files = Vec::new();
	for entry in glob::glob(
		path.as_ref()
			.join("*.geojson")
			.display()
			.to_string()
			.as_str(),
	)? {
		files.push(tokio::spawn(async { load_geojson(entry?).await }));
	}

	let gs = try_join_all(files)
		.await?
		.into_iter()
		.collect::<Result<Vec<_>, _>>()?
		.into_iter()
		.map(|gc| gc.0)
		.flatten()
		.collect::<Vec<_>>();

	Ok(GeometryCollection(gs))
}

#[cfg(debug_assertions)]
pub async fn debug_write_geojson(name: &str, polys: &MultiPolygon<f64>) -> Result<()> {
	use tokio::io::AsyncWriteExt;

	warn!(%name, "writing debug geojson");
	let gc = GeometryCollection::from(polys.clone());
	let gj = GeoJson::FeatureCollection(FeatureCollection::from(&gc)).to_string();
	File::create(format!("test-output.{}.geojson", name))
		.await?
		.write_all(gj.as_bytes())
		.await?;
	Ok(())
}

async fn load_geojson(path: PathBuf) -> Result<GeometryCollection<f64>> {
	debug!(?path, "reading geojson");

	let mut file = File::open(&path).await?;
	trace!(?path, ?file, "opened file");

	let bytes = usize::try_from(file.metadata().await?.len())?;
	trace!(?path, %bytes, "got file length");

	let mut contents = Vec::with_capacity(bytes);
	file.read_to_end(&mut contents).await?;
	debug!(?path, bytes=%contents.len(), "read geojson file");
	trace!(?path, ?contents, "file contents");

	let geo = GeoJson::from_reader(&contents[..])?;
	trace!(?path, ?geo, "parsed geojson");

	Ok(quick_collection(&geo)?)
}

pub fn only_polys(geometries: impl IntoIterator<Item = Geometry<f64>>) -> Vec<Polygon<f64>> {
	geometries
		.into_iter()
		.filter_map(|g| match g {
			Geometry::Polygon(p) => Some(vec![p]),
			Geometry::MultiPolygon(mp) => Some(mp.0),
			Geometry::GeometryCollection(gc) => Some(only_polys(gc)),
			_ => None,
		})
		.flatten()
		.collect()
}
