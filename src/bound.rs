use std::{convert::TryFrom, path::PathBuf};

use color_eyre::eyre::{eyre, Result};
use geo::{LineString, Coordinate, Polygon};
use geojson::{GeoJson, Geometry, PolygonType, Value};
use tokio::{fs::File, io::AsyncReadExt};
use tracing::{debug, trace};

pub async fn read_geojson(path: PathBuf) -> Result<Vec<Polygon<f64>>> {
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

	let polys = only_polys(match geo {
		GeoJson::FeatureCollection(fc) => fc.features.into_iter().filter_map(|f| f.geometry).collect(),
		GeoJson::Feature(f) => f.geometry.map(|g| vec![g]).unwrap_or_default(),
		GeoJson::Geometry(g) => vec![g],
	});
	trace!(?path, ?polys, "filtered to just polygons");

	let ps = polys.into_iter().map(|mut p| {
		let outer = line_string(p.remove(0))?;
		let inners = p.into_iter().map(line_string).collect::<Result<Vec<_>>>()?;
		let poly = Polygon::new(outer, inners);
		trace!(?path, ?poly, "converted (and validated) to geo::polygon");
		Ok(poly)
	}).collect::<Result<Vec<_>>>()?;

	debug!(?path, "obtained {} polygons", ps.len());

	Ok(ps)
}

fn only_polys(geometries: Vec<Geometry>) -> Vec<PolygonType> {
	geometries.into_iter().filter_map(|g| match g.value {
		Value::Polygon(p) => Some(vec![p]),
		Value::MultiPolygon(mp) => Some(mp),
		Value::GeometryCollection(gc) => Some(only_polys(gc)),
		_ => None,
	}).flatten().collect()
}

fn line_string(points: Vec<Vec<f64>>) -> Result<LineString<f64>> {
	let ls = LineString(points.into_iter().map(|p| Coordinate { x: p[0], y: p[1] }).collect());
	if !ls.is_closed() {
		Err(eyre!("line is not closed"))
	} else {
		Ok(ls)
	}
}
