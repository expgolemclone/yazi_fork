use std::{
	fs, io,
	path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use yazi_boot::BOOT;
use yazi_core::mgr::Favorites;
use yazi_shared::url::UrlBuf;

pub(crate) struct FavoritesIo;

impl FavoritesIo {
	pub(crate) fn path() -> PathBuf {
		BOOT.state_dir.join("favorites.json")
	}

	pub(crate) fn load() -> Result<Favorites> {
		Self::load_path(&Self::path())
	}

	pub(crate) fn save(favorites: &Favorites) -> Result<()> {
		Self::save_path(&Self::path(), favorites)
	}

	fn load_path(path: &Path) -> Result<Favorites> {
		let json = match fs::read_to_string(path) {
			Ok(json) => json,
			Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Default::default()),
			Err(err) => {
				return Err(err)
					.with_context(|| format!("Failed to read favorites file: {}", path.display()));
			}
		};

		let urls: Vec<UrlBuf> = serde_json::from_str(&json)
			.with_context(|| format!("Failed to parse favorites file: {}", path.display()))?;
		Ok(urls.into_iter().collect())
	}

	fn save_path(path: &Path, favorites: &Favorites) -> Result<()> {
		if let Some(parent) = path.parent() {
			fs::create_dir_all(parent)
				.with_context(|| format!("Failed to create favorites directory: {}", parent.display()))?;
		}

		let json = serde_json::to_string_pretty(&favorites.iter().collect::<Vec<_>>())
			.context("Failed to serialize favorites")?;
		fs::write(path, json)
			.with_context(|| format!("Failed to write favorites file: {}", path.display()))
	}
}

#[cfg(test)]
mod tests {
	use std::{
		fs,
		path::{Path, PathBuf},
	};

	use yazi_core::mgr::Favorites;

	use super::*;

	struct TempDir {
		path: PathBuf,
	}

	impl TempDir {
		fn new() -> Self {
			let path = std::env::temp_dir().join(format!(
				"yazi-favorites-test-{}-{}",
				std::process::id(),
				yazi_shared::timestamp_us()
			));
			fs::create_dir_all(&path).unwrap();
			Self { path }
		}

		fn child(&self, name: &str) -> PathBuf {
			self.path.join(name)
		}
	}

	impl Drop for TempDir {
		fn drop(&mut self) {
			let _ = fs::remove_dir_all(&self.path);
		}
	}

	fn save_path(path: &Path, favorites: &Favorites) {
		FavoritesIo::save_path(path, favorites).unwrap();
	}

	fn load_path(path: &Path) -> Result<Favorites> {
		FavoritesIo::load_path(path)
	}

	#[test]
	fn load_path_returns_empty_when_file_is_missing() {
		let dir = TempDir::new();

		let favorites = load_path(&dir.child("favorites.json")).unwrap();
		assert!(favorites.is_empty());
	}

	#[test]
	fn save_and_load_round_trip_json_string_array() {
		let dir = TempDir::new();
		let path = dir.child("favorites.json");
		let mut favorites = Favorites::default();

		favorites.set_many([Path::new("/a"), Path::new("/b")], true);
		save_path(&path, &favorites);

		assert_eq!("[\n  \"/a\",\n  \"/b\"\n]", fs::read_to_string(&path).unwrap());

		let loaded = load_path(&path).unwrap();
		assert_eq!(favorites, loaded);
	}

	#[test]
	fn load_path_fails_on_malformed_json() {
		let dir = TempDir::new();
		let path = dir.child("favorites.json");

		fs::write(&path, "{").unwrap();
		assert!(load_path(&path).is_err());
	}

	#[test]
	fn load_path_fails_on_invalid_url_entries() {
		let dir = TempDir::new();
		let path = dir.child("favorites.json");

		fs::write(&path, "[1]").unwrap();
		assert!(load_path(&path).is_err());
	}
}
