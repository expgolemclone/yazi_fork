use std::path::{Path, PathBuf};

use anyhow::Result;
use image::{
	DynamicImage, ImageDecoder, ImageError, ImageReader, Limits,
	codecs::{jpeg::JpegEncoder, png::PngEncoder},
	imageops::FilterType,
	metadata::Orientation,
};
use ratatui::layout::Rect;
use yazi_config::YAZI;
use yazi_emulator::Dimension;
use yazi_fs::provider::{Provider, local::Local};

use crate::Icc;

pub struct Image;

impl Image {
	pub async fn precache(src: PathBuf, cache: &Path) -> Result<()> {
		let (mut img, orientation) = Self::decode_from(src).await?;
		let (w, h) = Self::flip_size(orientation, (YAZI.preview.max_width, YAZI.preview.max_height));

		let buf = tokio::task::spawn_blocking(move || {
			if img.width() > w || img.height() > h {
				img = img.resize(w, h, Self::filter());
			}
			if orientation != Orientation::NoTransforms {
				img.apply_orientation(orientation);
			}

			let mut buf = Vec::new();
			if img.color().has_alpha() {
				let encoder = PngEncoder::new(&mut buf);
				img.write_with_encoder(encoder)?;
			} else {
				let encoder = JpegEncoder::new_with_quality(&mut buf, YAZI.preview.image_quality);
				img.write_with_encoder(encoder)?;
			}

			Ok::<_, ImageError>(buf)
		})
		.await??;

		Ok(Local::regular(&cache).write(buf).await?)
	}

	pub(super) async fn fit(path: PathBuf, rect: Rect) -> Result<DynamicImage> {
		let (mut img, orientation) = Self::decode_from(path).await?;
		let (w, h) = Self::max_pixel(rect);
		let target = Self::fit_size(
			Self::oriented_size(orientation, (img.width(), img.height())),
			(w.into(), h.into()),
		);
		let target = Self::oriented_size(orientation, target);

		// Fast path.
		if (img.width(), img.height()) == target && orientation == Orientation::NoTransforms {
			return Ok(img);
		}

		let img = tokio::task::spawn_blocking(move || {
			if (img.width(), img.height()) != target {
				img = img.resize(target.0, target.1, Self::filter())
			}
			if orientation != Orientation::NoTransforms {
				img.apply_orientation(orientation);
			}
			img
		})
		.await?;

		Ok(img)
	}

	pub(super) fn max_pixel(rect: Rect) -> (u16, u16) {
		Dimension::cell_size()
			.map(|cell| {
				Self::max_pixel_with_cell(rect, cell, (YAZI.preview.max_width, YAZI.preview.max_height))
			})
			.unwrap_or((YAZI.preview.max_width, YAZI.preview.max_height))
	}

	pub(super) fn pixel_area(size: (u32, u32), rect: Rect) -> Rect {
		Dimension::cell_size().map(|cell| Self::pixel_area_with_cell(size, rect, cell)).unwrap_or(rect)
	}

	pub(super) fn fit_area(size: (u32, u32), rect: Rect) -> Rect {
		let (w, h) = Self::max_pixel(rect);
		Self::pixel_area(Self::fit_size(size, (w.into(), h.into())), rect)
	}

	fn filter() -> FilterType {
		match YAZI.preview.image_filter.as_str() {
			"nearest" => FilterType::Nearest,
			"triangle" => FilterType::Triangle,
			"catmull-rom" => FilterType::CatmullRom,
			"gaussian" => FilterType::Gaussian,
			"lanczos3" => FilterType::Lanczos3,
			_ => FilterType::Triangle,
		}
	}

	async fn decode_from(path: PathBuf) -> Result<(DynamicImage, Orientation)> {
		let mut limits = Limits::no_limits();
		if YAZI.tasks.image_alloc > 0 {
			limits.max_alloc = Some(YAZI.tasks.image_alloc as u64);
		}
		if YAZI.tasks.image_bound[0] > 0 {
			limits.max_image_width = Some(YAZI.tasks.image_bound[0] as u32);
		}
		if YAZI.tasks.image_bound[1] > 0 {
			limits.max_image_height = Some(YAZI.tasks.image_bound[1] as u32);
		}

		tokio::task::spawn_blocking(move || {
			let mut reader = ImageReader::open(path)?;
			reader.limits(limits);

			let mut decoder = reader.with_guessed_format()?.into_decoder()?;
			let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
			Ok((Icc::transform(decoder)?, orientation))
		})
		.await
		.map_err(|e| ImageError::IoError(e.into()))?
	}

	fn fit_size(size: (u32, u32), bound: (u32, u32)) -> (u32, u32) {
		if size.0 == 0 || size.1 == 0 || bound.0 == 0 || bound.1 == 0 {
			return (0, 0);
		}

		let wratio = bound.0 as f64 / size.0 as f64;
		let hratio = bound.1 as f64 / size.1 as f64;
		let ratio = wratio.min(hratio);

		let nw = ((size.0 as f64 * ratio).round() as u64).max(1);
		let nh = ((size.1 as f64 * ratio).round() as u64).max(1);

		if nw > u64::from(u32::MAX) {
			let ratio = u32::MAX as f64 / size.0 as f64;
			(u32::MAX, ((size.1 as f64 * ratio).round() as u32).max(1))
		} else if nh > u64::from(u32::MAX) {
			let ratio = u32::MAX as f64 / size.1 as f64;
			(((size.0 as f64 * ratio).round() as u32).max(1), u32::MAX)
		} else {
			(nw as u32, nh as u32)
		}
	}

	fn max_pixel_with_cell(rect: Rect, (cw, ch): (f64, f64), bound: (u16, u16)) -> (u16, u16) {
		let (w, h) = ((rect.width as f64 * cw) as u16, (rect.height as f64 * ch) as u16);
		(w.min(bound.0), h.min(bound.1))
	}

	fn pixel_area_with_cell(size: (u32, u32), rect: Rect, (cw, ch): (f64, f64)) -> Rect {
		Rect {
			x: rect.x,
			y: rect.y,
			width: ((size.0 as f64 / cw).ceil() as u16).min(rect.width),
			height: ((size.1 as f64 / ch).ceil() as u16).min(rect.height),
		}
	}

	pub(super) fn oriented_size(orientation: Orientation, (w, h): (u32, u32)) -> (u32, u32) {
		use image::metadata::Orientation::{Rotate90, Rotate90FlipH, Rotate270, Rotate270FlipH};
		match orientation {
			Rotate90 | Rotate270 | Rotate90FlipH | Rotate270FlipH => (h, w),
			_ => (w, h),
		}
	}

	fn flip_size(orientation: Orientation, (w, h): (u16, u16)) -> (u32, u32) {
		Self::oriented_size(orientation, (w as u32, h as u32))
	}
}

#[cfg(test)]
mod tests {
	use ratatui::layout::Rect;

	use super::Image;

	#[test]
	fn fit_size_upscales_small_images() {
		assert_eq!(Image::fit_size((50, 50), (200, 200)), (200, 200));
	}

	#[test]
	fn fit_size_downscales_large_images() {
		assert_eq!(Image::fit_size((400, 100), (200, 200)), (200, 50));
	}

	#[test]
	fn fit_area_preserves_aspect_ratio_for_wide_images() {
		let rect = Rect { x: 4, y: 2, width: 20, height: 10 };
		let area =
			Image::pixel_area_with_cell(Image::fit_size((400, 100), (200, 100)), rect, (10.0, 10.0));

		assert_eq!(area, Rect { x: 4, y: 2, width: 20, height: 5 });
	}

	#[test]
	fn fit_area_preserves_aspect_ratio_for_tall_images() {
		let rect = Rect { x: 1, y: 1, width: 20, height: 10 };
		let area =
			Image::pixel_area_with_cell(Image::fit_size((100, 400), (200, 100)), rect, (10.0, 10.0));

		assert_eq!(area, Rect { x: 1, y: 1, width: 3, height: 10 });
	}

	#[test]
	fn max_pixel_respects_preview_limits() {
		let rect = Rect { x: 0, y: 0, width: 100, height: 100 };
		assert_eq!(Image::max_pixel_with_cell(rect, (10.0, 10.0), (150, 120)), (150, 120));
	}

	#[test]
	fn pixel_area_clamps_to_preview_rect() {
		let rect = Rect { x: 0, y: 0, width: 20, height: 10 };
		let area = Image::pixel_area_with_cell((201, 101), rect, (10.0, 10.0));

		assert_eq!(area, Rect { x: 0, y: 0, width: 20, height: 10 });
	}
}
