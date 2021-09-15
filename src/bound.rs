use std::path::PathBuf;

use color_eyre::eyre::Result;
use geo::Polygon;

pub async fn read_geojson(_file: PathBuf) -> Result<Vec<Polygon<f64>>> {
	todo!()
}
