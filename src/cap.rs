use std::{
	collections::HashMap,
	hash::{Hash, Hasher},
	iter::FromIterator,
	num::ParseFloatError,
	str::FromStr,
};

use chrono::{DateTime, Utc};
use color_eyre::eyre::{eyre, Result};
use geo::{CoordFloat, CoordNum, Coordinate, GeometryCollection, LineString, Polygon};
use geojson::{FeatureCollection, GeoJson};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tracing::{debug, error, info, trace};

use crate::feed::Item;

pub async fn fetch_cap(item: Item) -> Result<Cap> {
	let guid = item.guid;
	info!(%guid, "fetching CAP");
	let resp = reqwest::get(&item.link).await?;

	if !resp.status().is_success() {
		error!(status=%resp.status(), "failed to fetch CAP");
		return Err(eyre!("failed to fetch CAP {}", guid));
	}

	info!(
		%guid,
		age=%resp.headers().get("age").map(|v| v.to_str()).transpose()?.unwrap_or("?"),
		bytes=%resp.headers().get("content-length").map(|v| v.to_str()).transpose()?.unwrap_or("?"),
		"got CAP"
	);
	trace!(%guid, status=?resp.status(), headers=?resp.headers(), "headers");

	let body = resp.text().await?;
	debug!(%guid, chars=%body.len(), "decoded body as text");
	trace!(%guid, body=%body, "decoded body");

	let cap: Cap = serde_xml_rs::from_str(&body)?;
	trace!(%guid, ?cap, "parsed cap");

	info!(
		%guid,
		about=%cap.info.headline,
		areas=?cap.info.areas.iter().map(|a| &a.desc).collect::<Vec<_>>(),
		"parsed cap"
	);

	Ok(cap)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Cap {
	#[serde(rename = "identifier")]
	pub guid: String,

	#[serde(rename = "sent")]
	pub date_sent: DateTime<Utc>,

	pub status: String,
	pub scope: String,

	#[serde(rename = "msgType")]
	pub msg_type: String,

	pub info: Info,
}

impl Hash for Cap {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.guid.hash(state);
	}
}

impl PartialEq<Self> for Cap {
	fn eq(&self, other: &Self) -> bool {
		self.guid == other.guid
	}
}

impl Eq for Cap {}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Info {
	pub category: String,
	pub event: String,
	pub urgency: String,
	pub severity: Severity,
	pub certainty: String,
	pub onset: DateTime<Utc>,
	pub expires: DateTime<Utc>,
	pub headline: String,
	pub description: String,
	pub instruction: String,

	#[serde(rename = "responseType")]
	pub response_type: String,

	#[serde(rename = "senderName")]
	pub sender_name: String,

	#[serde(rename = "parameter", deserialize_with = "parameters_de")]
	pub parameters: HashMap<String, String>,

	#[serde(rename = "area")]
	pub areas: Vec<Area>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
	Minor,
	Moderate,
	Severe,
	Extreme,
}

impl FromStr for Severity {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"minor" => Ok(Self::Minor),
			"moderate" => Ok(Self::Moderate),
			"severe" => Ok(Self::Severe),
			"extreme" => Ok(Self::Extreme),
			_ => Err(format!("invalid severity: {}", s)),
		}
	}
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Area {
	#[serde(rename = "areaDesc")]
	pub desc: String,

	#[serde(
		rename = "polygon",
		deserialize_with = "polygons_de",
		serialize_with = "polygons_ser"
	)]
	pub polygons: Vec<Polygon<f64>>,
}

fn parameters_de<'de, D>(deserializer: D) -> Result<HashMap<String, String>, D::Error>
where
	D: Deserializer<'de>,
{
	#[derive(Deserialize)]
	struct Parameter {
		#[serde(rename = "valueName")]
		name: String,
		value: String,
	}

	let params = Vec::<Parameter>::deserialize(deserializer)?;
	let mut map = HashMap::new();
	for param in params {
		map.insert(param.name, param.value);
	}
	Ok(map)
}

fn polygons_de<'de, T, D>(deserializer: D) -> Result<Vec<Polygon<T>>, D::Error>
where
	T: CoordNum + FromStr<Err = ParseFloatError>,
	D: Deserializer<'de>,
{
	let texts = Vec::<String>::deserialize(deserializer)?;
	trace!("parsing {} polygons", texts.len());
	texts
		.into_iter()
		.map(polygon::<'de, T, D>)
		.collect::<Result<_, _>>()
}

fn polygon<'de, T, D>(text: String) -> Result<Polygon<T>, D::Error>
where
	T: CoordNum + FromStr<Err = ParseFloatError>,
	D: Deserializer<'de>,
{
	use serde::de::Error;

	trace!(%text, "parsing polygon");

	let coords = text
		.split_whitespace()
		.map(|s| {
			let (y, x) = s
				.split_once(',')
				.ok_or_else(|| Error::custom("invalid coordinate pair"))?;
			let x = x.parse::<T>().map_err(Error::custom)?;
			let y = y.parse::<T>().map_err(Error::custom)?;
			Ok(Coordinate { x, y })
		})
		.collect::<Result<Vec<_>, D::Error>>()?;
	trace!(?coords, "parsed bunch of coordinates");

	let line = LineString(coords);
	if !line.is_closed() {
		trace!(?line, "polygon is not closed");
		return Err(Error::custom("polygon is not closed"));
	}

	Ok(Polygon::new(line, Vec::new()))
}

fn polygons_ser<S, T>(polys: &[Polygon<T>], serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
	T: CoordNum + CoordFloat,
{
	let gc = GeometryCollection::<T>::from_iter(polys.to_owned());
	let fc = FeatureCollection::from(&gc);
	let geojson = GeoJson::FeatureCollection(fc);
	geojson.serialize(serializer)
}
