use std::collections::HashSet;

use color_eyre::eyre::{eyre, Result};
use geo::concave_hull::ConcaveHull;
use geo::prelude::{BoundingRect, Contains};
use geo::{Geometry, MultiPolygon};
use geo_booleanop::boolean::BooleanOp;
use geozero::ToSvg;
use itertools::Itertools;
use tiny_skia::Pixmap;
use tracing::{debug, trace};
use usvg::{FitTo, Options, Size, Tree};

use super::{text, Out};
use crate::cap::Cap;
use crate::geodirs::load_polygons;
use crate::Args;

pub async fn text_with_map(caps: HashSet<Cap>, args: &Args) -> Result<Out> {
	debug!("loading boundaries");
	let boundaries = MultiPolygon(
		load_polygons(&args.boundaries)
			.await?
			.0
			.into_iter()
			.fold1(|a, b| a.union(&b).concave_hull(2.0))
			.map(|p| vec![p])
			.unwrap_or_default(),
	);

	let mut areas = caps
		.iter()
		.map(|c| c.info.areas.clone())
		.flatten()
		.map(|a| a.polygons)
		.flatten()
		.collect::<MultiPolygon<f64>>();

	let bbox = boundaries
		.bounding_rect()
		.or_else(|| areas.bounding_rect())
		.ok_or_else(|| eyre!("cannot get bounding box"))?;
	debug!(?bbox, "bounding box of map");

	if !boundaries.0.is_empty() && !boundaries.contains(&areas) {
		debug!("cropping areas to boundaries");
		areas = areas.intersection(&boundaries);
	}

	#[cfg(debug_assertions)]
	crate::geodirs::debug_write_geojson("area", &areas).await?;

	let mut mps = vec![Mps {
		mp: &areas,
		stroke: "#9900ff",
		width: 0.01,
		fill: "#9900ff",
		opacity: 0.5,
	}];

	debug!("loading outlines");
	let mut outlines = load_polygons(&args.outlines).await?;
	if !outlines.0.is_empty() {
		if !boundaries.0.is_empty() && !boundaries.contains(&outlines) {
			debug!("cropping outlines to boundaries");
			outlines = outlines.intersection(&boundaries);
		}

		#[cfg(debug_assertions)]
		crate::geodirs::debug_write_geojson("outline", &outlines).await?;

		mps.insert(
			0,
			Mps {
				mp: &outlines,
				stroke: "black",
				width: 0.005,
				fill: "#ffeebb",
				opacity: 1.0,
			},
		);
	}

	debug!("translating to svg");
	let svg = mps_to_svg(&mps, "#88ddff")?;

	trace!(%svg, "svg string");
	#[cfg(debug_assertions)]
	debug_file("test-output.svg", svg.as_bytes()).await?;

	debug!("reparsing svg into usvg");
	let opts = Options {
		default_size: Size::new(bbox.width(), bbox.height())
			.ok_or_else(|| eyre!("cannot create zero-sized svg"))?,
		..Options::default()
	};
	let svg = Tree::from_str(&svg, &opts.to_ref())?;
	trace!(svg=%svg.root().traverse().map(|n| format!("{:?}", n)).collect::<Vec<_>>().join("\n"), "usvg tree");

	debug!("rendering");
	let mut pix = Pixmap::new(args.image_width, args.image_height)
		.ok_or_else(|| eyre!("Cannot create zero-sized image"))?;
	trace!(?pix, "created image buffer");

	debug!("rendering svg to image buffer");
	resvg::render(
		&svg,
		FitTo::Size(args.image_width, args.image_height),
		pix.as_mut(),
	)
	.ok_or_else(|| eyre!("Failed to render image"))?;
	let (_, _, pix) =
		resvg::trim_transparency(pix).ok_or_else(|| eyre!("Failed to trim transparency"))?;

	debug!("encoding as png");
	let image = pix.encode_png()?;
	debug!(bytes=%image.len(), "encoded");

	let mut out = text(caps)?;
	out.image = Some(image);
	Ok(out)
}

#[derive(Clone, Debug)]
struct Mps<'a> {
	mp: &'a MultiPolygon<f64>,
	stroke: &'a str,
	fill: &'a str,
	width: f32,
	opacity: f32,
}

fn mps_to_svg(mps: &[Mps], background: &str) -> Result<String> {
	let bbox = MultiPolygon(
		mps.iter()
			.map(|m| m.mp.concave_hull(2.0))
			.collect::<Vec<_>>(),
	)
	.concave_hull(2.0)
	.bounding_rect()
	.ok_or_else(|| eyre!("cannot get bbox for multipolygon"))?;

	Ok(format!(
		r#"<svg
			xmlns="http://www.w3.org/2000/svg"
			xmlns:xlink="http://www.w3.org/1999/xlink"
			version="1.2"
			viewBox="0 0 {w} {h}"
		>
			<rect x="0" y="0" width="{w}" height="{h}" fill="{bg}" />
			<g transform="translate({negx},{negy}) scale(1,-1) translate(0,{transup})">
				{layers}
			</g>
		</svg>"#,
		negx = -bbox.min().x,
		negy = -bbox.min().y,
		w = bbox.width(),
		h = bbox.height(),
		transup = ((-bbox.min().y) * 2.0 - bbox.height()),
		bg = background,
		layers = mps
			.iter()
			.map(|m| {
				Ok(format!(
					r#"<g stroke="{}" stroke-width="{}" fill="{}" fill-opacity="{}">{}</g>"#,
					m.stroke,
					m.width,
					m.fill,
					m.opacity,
					Geometry::from(m.mp.clone()).to_svg()?
				))
			})
			.collect::<Result<Vec<_>>>()?
			.join("\n")
	))
}

#[cfg(debug_assertions)]
async fn debug_file(name: &str, data: &[u8]) -> Result<()> {
	use tokio::{fs::File, io::AsyncWriteExt};

	tracing::warn!(%name, "writing debug file");
	File::create(name).await?.write_all(data).await?;

	Ok(())
}
