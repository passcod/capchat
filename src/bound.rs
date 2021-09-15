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

pub async fn read_geojson(file: PathBuf) -> Result<Vec<Polygon<f64>>> {
	todo!()
}
