use std::path::PathBuf;

use anyhow::Result;
use arc_swap::ArcSwap;
use serde::{Deserialize, Deserializer, de};
use yazi_codegen::{DeserializeOver, DeserializeOver2};
use yazi_fs::{SortBy, SortFallback};
use yazi_shim::{arc_swap::IntoPointee, cell::SyncCell};

use super::{MgrRatio, MouseEvents};
use crate::normalize_path;

#[derive(Debug, Deserialize, DeserializeOver, DeserializeOver2)]
pub struct Mgr {
	pub ratio: SyncCell<MgrRatio>,

	// Sorting
	pub sort_by: SyncCell<SortBy>,
	pub sort_sensitive: SyncCell<bool>,
	pub sort_reverse: SyncCell<bool>,
	pub sort_dir_first: SyncCell<bool>,
	pub sort_translit: SyncCell<bool>,
	pub sort_fallback: SyncCell<SortFallback>,

	// Display
	#[serde(deserialize_with = "deserialize_linemode")]
	pub linemode: ArcSwap<String>,
	pub show_hidden: SyncCell<bool>,
	pub show_symlink: SyncCell<bool>,
	pub scrolloff: SyncCell<u8>,
	pub mouse_events: SyncCell<MouseEvents>,
	#[serde(default, deserialize_with = "deserialize_favorites_file")]
	pub favorites_file: PathBuf,
}

fn deserialize_linemode<'de, D>(deserializer: D) -> Result<ArcSwap<String>, D::Error>
where
	D: Deserializer<'de>,
{
	let s = String::deserialize(deserializer)?;
	if s.is_empty() || s.len() > 20 {
		return Err(de::Error::custom("linemode must be between 1 and 20 characters."));
	}

	Ok(s.into_pointee())
}

fn deserialize_favorites_file<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
where
	D: Deserializer<'de>,
{
	let path = PathBuf::deserialize(deserializer)?;
	if path.as_os_str().is_empty() {
		return Ok(path);
	}

	if path.is_absolute() || path.starts_with("~") {
		normalize_path(path)
			.ok_or_else(|| de::Error::custom("favorites_file must be either empty or an absolute path."))
	} else {
		Err(de::Error::custom("favorites_file must be either empty or an absolute path."))
	}
}

#[cfg(test)]
mod tests {
	use std::path::PathBuf;

	use super::Mgr;

	fn mgr_toml(favorites_file: &str) -> String {
		format!(
			r#"
ratio = [1, 4, 3]
sort_by = "alphabetical"
sort_sensitive = false
sort_reverse = false
sort_dir_first = true
sort_translit = false
sort_fallback = "alphabetical"
linemode = "none"
show_hidden = false
show_symlink = true
scrolloff = 5
mouse_events = ["click", "scroll", "drag"]
favorites_file = "{favorites_file}"
"#
		)
	}

	#[test]
	fn favorites_file_allows_empty_string() {
		let mgr: Mgr = toml::from_str(&mgr_toml("")).unwrap();
		assert!(mgr.favorites_file.as_os_str().is_empty());
	}

	#[test]
	fn favorites_file_accepts_absolute_paths() {
		let path = std::env::temp_dir().join("favorites.json");
		let path = path.to_string_lossy().replace('\\', "\\\\");

		let mgr: Mgr = toml::from_str(&mgr_toml(&path)).unwrap();
		assert_eq!(mgr.favorites_file, PathBuf::from(path.replace("\\\\", "\\")));
	}

	#[cfg(unix)]
	#[test]
	fn favorites_file_expands_home_paths() {
		let home = PathBuf::from(std::env::var_os("HOME").unwrap());
		let mgr: Mgr = toml::from_str(&mgr_toml("~/favorites.json")).unwrap();

		assert_eq!(mgr.favorites_file, home.join("favorites.json"));
	}

	#[test]
	fn favorites_file_rejects_relative_paths() {
		let err = toml::from_str::<Mgr>(&mgr_toml("state/favorites.json")).unwrap_err();
		assert!(err.to_string().contains("favorites_file must be either empty or an absolute path."));
	}
}
