use std::ops::Deref;

use indexmap::IndexSet;
use yazi_fs::FilesOp;
use yazi_shared::url::{Url, UrlBuf, UrlBufCov, UrlCov, UrlLike};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FavoriteCycle {
	anchor: Option<UrlBuf>,
	current: Option<UrlBuf>,
	prev: Option<UrlBuf>,
	next: Option<UrlBuf>,
}

impl FavoriteCycle {
	pub fn matches(&self, url: &UrlBuf) -> bool {
		self.current.as_ref().or(self.anchor.as_ref()).is_some_and(|current| current == url)
	}

	pub fn target(&self, prev: bool) -> Option<&UrlBuf> {
		if prev { self.prev.as_ref() } else { self.next.as_ref() }
	}

	pub fn advance(&mut self, favorites: &Favorites, anchor: UrlBuf) {
		self.current = Some(anchor.clone());
		self.anchor = Some(anchor);
		self.recenter(favorites);
	}

	pub fn relocate(&mut self, current: UrlBuf) {
		if self.anchor.is_some() {
			self.current = Some(current);
		}
	}

	pub fn reconcile(&mut self, before: &Favorites, after: &Favorites) {
		let Some(anchor) = self.anchor.clone() else { return };
		if after.is_empty() {
			self.prev = None;
			self.next = None;
			return;
		}

		if after.contains(&anchor) {
			self.recenter(after);
			return;
		}
		if before.contains(&anchor) {
			self.prev = Self::survivor(before, after, &anchor, true).or_else(|| Self::last(after));
			self.next = Self::survivor(before, after, &anchor, false).or_else(|| Self::first(after));
			return;
		}

		self.prev =
			self.prev.as_ref().filter(|url| after.contains(*url)).cloned().or_else(|| Self::last(after));
		self.next =
			self.next.as_ref().filter(|url| after.contains(*url)).cloned().or_else(|| Self::first(after));
	}

	pub fn rename_refs(&mut self, op: &FilesOp) {
		for slot in [&mut self.anchor, &mut self.current, &mut self.prev, &mut self.next] {
			let Some(url) = slot.as_ref() else { continue };
			if let Some(new) = Self::renamed(url, op) {
				*slot = Some(new);
			}
		}
	}

	fn recenter(&mut self, favorites: &Favorites) {
		let Some(anchor) = self.anchor.as_ref() else { return };

		self.prev = favorites.arrow(anchor, true).cloned();
		self.next = favorites.arrow(anchor, false).cloned();
	}

	fn survivor(
		before: &Favorites,
		after: &Favorites,
		anchor: &UrlBuf,
		prev: bool,
	) -> Option<UrlBuf> {
		let sorted = before.sorted();
		let len = sorted.len();
		let current = sorted.iter().position(|u| UrlCov::from(*u) == UrlCov::from(anchor))?;

		for step in 1..len {
			let idx = if prev { (current + len - step) % len } else { (current + step) % len };
			let candidate = sorted.get(idx)?;
			if after.contains(*candidate) {
				return Some((*candidate).clone());
			}
		}
		None
	}

	fn renamed(url: &UrlBuf, op: &FilesOp) -> Option<UrlBuf> {
		let map = match op {
			FilesOp::Updating(cwd, map) | FilesOp::Upserting(cwd, map) => Some((cwd, map)),
			_ => None,
		}?;

		map.1.iter().filter(|&(urn, file)| urn != &file.urn()).find_map(|(urn, file)| {
			map.0.try_join(urn).ok().filter(|old| old == url).map(|_| file.url_owned())
		})
	}

	fn first(favorites: &Favorites) -> Option<UrlBuf> {
		favorites.sorted().into_iter().next().cloned()
	}

	fn last(favorites: &Favorites) -> Option<UrlBuf> {
		favorites.sorted().into_iter().next_back().cloned()
	}
}

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

	fn sorted(&self) -> Vec<&UrlBuf> {
		let mut v: Vec<_> = self.urls.iter().map(Deref::deref).collect();
		v.sort();
		v
	}

	pub fn arrow<'a>(&self, url: impl Into<Url<'a>>, prev: bool) -> Option<&UrlBuf> {
		if self.urls.is_empty() {
			return None;
		}

		let cov = UrlCov::new(url);
		let sorted = self.sorted();
		let len = sorted.len();
		let current = sorted.iter().position(|u| UrlCov::from(*u) == cov);
		let next = match (current, prev) {
			(Some(0), true) => len - 1,
			(Some(i), true) => i - 1,
			(Some(i), false) => (i + 1) % len,
			(None, true) => len - 1,
			(None, false) => 0,
		};

		sorted.into_iter().nth(next)
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
	fn arrow_moves_forward_and_wraps() {
		let favorites = Favorites::from_iter([
			Path::new("/a").into(),
			Path::new("/b").into(),
			Path::new("/c").into(),
		]);
		let b: UrlBuf = Path::new("/b").into();
		let a: UrlBuf = Path::new("/a").into();

		assert_eq!(Some(&b), favorites.arrow(Path::new("/a"), false));
		assert_eq!(Some(&a), favorites.arrow(Path::new("/c"), false));
	}

	#[test]
	fn arrow_moves_backward_and_wraps() {
		let favorites = Favorites::from_iter([
			Path::new("/a").into(),
			Path::new("/b").into(),
			Path::new("/c").into(),
		]);
		let c: UrlBuf = Path::new("/c").into();
		let b: UrlBuf = Path::new("/b").into();

		assert_eq!(Some(&c), favorites.arrow(Path::new("/a"), true));
		assert_eq!(Some(&b), favorites.arrow(Path::new("/c"), true));
	}

	#[test]
	fn arrow_uses_edges_when_current_is_not_favorited() {
		let favorites = Favorites::from_iter([Path::new("/a").into(), Path::new("/b").into()]);
		let a: UrlBuf = Path::new("/a").into();
		let b: UrlBuf = Path::new("/b").into();

		assert_eq!(Some(&a), favorites.arrow(Path::new("/z"), false));
		assert_eq!(Some(&b), favorites.arrow(Path::new("/z"), true));
	}

	#[test]
	fn arrow_returns_same_item_for_single_favorite() {
		let favorites = Favorites::from_iter([Path::new("/only").into()]);
		let only: UrlBuf = Path::new("/only").into();

		assert_eq!(Some(&only), favorites.arrow(Path::new("/only"), false));
		assert_eq!(Some(&only), favorites.arrow(Path::new("/other"), true));
	}

	#[test]
	fn arrow_returns_none_when_empty() {
		assert_eq!(None, Favorites::default().arrow(Path::new("/any"), false));
	}

	#[test]
	fn arrow_uses_sorted_order_regardless_of_insertion_order() {
		let favorites = Favorites::from_iter([
			Path::new("/c").into(),
			Path::new("/a").into(),
			Path::new("/b").into(),
		]);
		let a: UrlBuf = Path::new("/a").into();
		let b: UrlBuf = Path::new("/b").into();
		let c: UrlBuf = Path::new("/c").into();

		assert_eq!(Some(&b), favorites.arrow(Path::new("/a"), false));
		assert_eq!(Some(&c), favorites.arrow(Path::new("/b"), false));
		assert_eq!(Some(&a), favorites.arrow(Path::new("/c"), false));
		assert_eq!(Some(&c), favorites.arrow(Path::new("/a"), true));
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

	#[test]
	fn cycle_keeps_neighbors_when_anchor_is_removed() {
		let before = Favorites::from_iter([
			Path::new("/a").into(),
			Path::new("/b").into(),
			Path::new("/c").into(),
			Path::new("/d").into(),
		]);
		let after = Favorites::from_iter([
			Path::new("/a").into(),
			Path::new("/b").into(),
			Path::new("/d").into(),
		]);
		let removed: UrlBuf = Path::new("/c").into();
		let b: UrlBuf = Path::new("/b").into();
		let d: UrlBuf = Path::new("/d").into();

		let mut cycle = FavoriteCycle::default();
		cycle.advance(&before, removed.clone());
		cycle.reconcile(&before, &after);

		assert!(cycle.matches(&removed));
		assert_eq!(Some(&b), cycle.target(true));
		assert_eq!(Some(&d), cycle.target(false));
	}

	#[test]
	fn cycle_matches_relocated_current_without_recentering() {
		let favorites = Favorites::from_iter([
			Path::new("/a").into(),
			Path::new("/b").into(),
			Path::new("/c").into(),
		]);
		let a: UrlBuf = Path::new("/a").into();
		let b: UrlBuf = Path::new("/b").into();
		let b2: UrlBuf = Path::new("/alt/b").into();
		let c: UrlBuf = Path::new("/c").into();

		let mut cycle = FavoriteCycle::default();
		cycle.advance(&favorites, b.clone());
		cycle.relocate(b2.clone());

		assert!(!cycle.matches(&b));
		assert!(cycle.matches(&b2));
		assert_eq!(Some(&a), cycle.target(true));
		assert_eq!(Some(&c), cycle.target(false));
	}

	#[test]
	fn cycle_recenters_when_anchor_stays_favorited() {
		let before = Favorites::from_iter([
			Path::new("/a").into(),
			Path::new("/b").into(),
			Path::new("/c").into(),
		]);
		let after = Favorites::from_iter([Path::new("/a").into(), Path::new("/b").into()]);
		let b: UrlBuf = Path::new("/b").into();
		let a: UrlBuf = Path::new("/a").into();

		let mut cycle = FavoriteCycle::default();
		cycle.advance(&before, b.clone());
		cycle.reconcile(&before, &after);

		assert!(cycle.matches(&b));
		assert_eq!(Some(&a), cycle.target(true));
		assert_eq!(Some(&a), cycle.target(false));
	}

	#[test]
	fn cycle_renames_anchor_before_recentering() {
		let before = Favorites::from_iter([Path::new("/a").into(), Path::new("/b").into()]);
		let after = Favorites::from_iter([Path::new("/a").into(), Path::new("/b2").into()]);
		let b: UrlBuf = Path::new("/b").into();
		let b2: UrlBuf = Path::new("/b2").into();
		let a: UrlBuf = Path::new("/a").into();

		let mut files = HashMap::new();
		files.insert(PathBufDyn::from(Path::new("b")), File::from_dummy(Path::new("/b2"), None));

		let mut cycle = FavoriteCycle::default();
		cycle.advance(&before, b);
		cycle.rename_refs(&FilesOp::Upserting(Path::new("/").into(), files));
		cycle.reconcile(&before, &after);

		assert!(cycle.matches(&b2));
		assert_eq!(Some(&a), cycle.target(true));
		assert_eq!(Some(&a), cycle.target(false));
	}

	#[test]
	fn cycle_renames_neighbors_when_anchor_is_already_gone() {
		let before = Favorites::from_iter([
			Path::new("/a").into(),
			Path::new("/b").into(),
			Path::new("/d").into(),
		]);
		let after = Favorites::from_iter([
			Path::new("/a").into(),
			Path::new("/b").into(),
			Path::new("/d2").into(),
		]);
		let c: UrlBuf = Path::new("/c").into();
		let b: UrlBuf = Path::new("/b").into();
		let d2: UrlBuf = Path::new("/d2").into();

		let mut files = HashMap::new();
		files.insert(PathBufDyn::from(Path::new("d")), File::from_dummy(Path::new("/d2"), None));

		let mut cycle = FavoriteCycle {
			anchor: Some(c.clone()),
			current: Some(c.clone()),
			prev: Some(b.clone()),
			next: Some(Path::new("/d").into()),
		};
		cycle.rename_refs(&FilesOp::Upserting(Path::new("/").into(), files));
		cycle.reconcile(&before, &after);

		assert!(cycle.matches(&c));
		assert_eq!(Some(&b), cycle.target(true));
		assert_eq!(Some(&d2), cycle.target(false));
	}
}
