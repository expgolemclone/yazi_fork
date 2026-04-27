use std::ops::Deref;

use indexmap::IndexSet;
use yazi_fs::FilesOp;
use yazi_shared::url::{Url, UrlBuf, UrlBufCov, UrlCov};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Favorites {
	urls: IndexSet<UrlBufCov>,
}

impl Deref for Favorites {
	type Target = IndexSet<UrlBufCov>;

	fn deref(&self) -> &Self::Target {
		&self.urls
	}
}

impl Favorites {
	pub fn len(&self) -> usize {
		self.urls.len()
	}

	pub fn is_empty(&self) -> bool {
		self.urls.is_empty()
	}

	pub fn iter(&self) -> impl Iterator<Item = &UrlBuf> {
		self.urls.iter().map(Deref::deref)
	}

	pub fn contains<'a>(&self, url: impl Into<Url<'a>>) -> bool {
		self.urls.contains(&UrlCov::new(url))
	}

	pub fn set_many<'a, I, T>(&mut self, urls: I, state: bool) -> usize
	where
		I: IntoIterator<Item = T>,
		T: Into<Url<'a>>,
	{
		Self::unique_urls(urls)
			.into_iter()
			.map(
				|url| {
					if state { self.urls.insert(url) } else { self.urls.shift_remove(&UrlCov::from(&url)) }
				},
			)
			.map(usize::from)
			.sum()
	}

	pub fn toggle_many<'a, I, T>(&mut self, urls: I) -> usize
	where
		I: IntoIterator<Item = T>,
		T: Into<Url<'a>>,
	{
		Self::unique_urls(urls)
			.into_iter()
			.map(
				|url| {
					if self.urls.shift_remove(&UrlCov::from(&url)) { true } else { self.urls.insert(url) }
				},
			)
			.map(usize::from)
			.sum()
	}

	pub fn apply_op(&mut self, op: &FilesOp) -> bool {
		let (removal, addition) = op.diff_recoverable(|url| self.contains(url));
		let mut changed = false;

		for url in removal {
			changed |= self.urls.shift_remove(&UrlCov::from(&url));
		}
		for url in addition {
			changed |= self.urls.insert(url.into());
		}

		changed
	}

	fn unique_urls<'a, I, T>(urls: I) -> IndexSet<UrlBufCov>
	where
		I: IntoIterator<Item = T>,
		T: Into<Url<'a>>,
	{
		urls.into_iter().map(|url| UrlBufCov::from(url.into())).collect()
	}
}

impl FromIterator<UrlBuf> for Favorites {
	fn from_iter<T: IntoIterator<Item = UrlBuf>>(iter: T) -> Self {
		Self { urls: iter.into_iter().map(UrlBufCov::from).collect() }
	}
}

#[cfg(test)]
mod tests {
	use std::path::Path;

	use hashbrown::{HashMap, HashSet};
	use yazi_fs::File;
	use yazi_shared::path::PathBufDyn;

	use super::*;

	#[test]
	fn set_many_is_idempotent() {
		let mut favorites = Favorites::default();

		assert_eq!(2, favorites.set_many([Path::new("/a"), Path::new("/b")], true));
		assert_eq!(0, favorites.set_many([Path::new("/a"), Path::new("/b")], true));
		assert_eq!(2, favorites.len());
		assert!(favorites.contains(Path::new("/a")));
		assert!(favorites.contains(Path::new("/b")));
	}

	#[test]
	fn toggle_many_adds_and_removes_targets() {
		let mut favorites = Favorites::default();

		assert_eq!(2, favorites.toggle_many([Path::new("/a"), Path::new("/b")]));
		assert_eq!(1, favorites.toggle_many([Path::new("/a")]));
		assert!(favorites.contains(Path::new("/b")));
		assert!(!favorites.contains(Path::new("/a")));
	}

	#[test]
	fn apply_op_renames_favorited_paths() {
		let mut favorites = Favorites::default();
		favorites.set_many([Path::new("/old"), Path::new("/keep")], true);

		let mut files = HashMap::new();
		files.insert(PathBufDyn::from(Path::new("old")), File::from_dummy(Path::new("/new"), None));

		assert!(favorites.apply_op(&FilesOp::Upserting(Path::new("/").into(), files)));
		assert!(favorites.contains(Path::new("/new")));
		assert!(favorites.contains(Path::new("/keep")));
		assert!(!favorites.contains(Path::new("/old")));
	}

	#[test]
	fn apply_op_deletes_missing_favorites() {
		let mut favorites = Favorites::default();
		favorites.set_many([Path::new("/gone"), Path::new("/keep")], true);

		let urns = HashSet::from_iter([PathBufDyn::from(Path::new("gone"))]);
		assert!(favorites.apply_op(&FilesOp::Deleting(Path::new("/").into(), urns)));
		assert!(!favorites.contains(Path::new("/gone")));
		assert!(favorites.contains(Path::new("/keep")));
	}
}
