use anyhow::Result;
use yazi_macro::{render, succ};
use yazi_parser::mgr::FavoriteForm;
use yazi_shared::{data::Data, url::UrlBuf};

use super::FavoritesIo;
use crate::{Actor, Ctx};

pub struct Favorite;

impl Actor for Favorite {
	type Form = FavoriteForm;

	const NAME: &str = "favorite";

	fn act(cx: &mut Ctx, form: Self::Form) -> Result<Data> {
		let targets = resolve_targets(
			&form.urls,
			cx.tab().selected.values(),
			cx.hovered().map(|hovered| &hovered.url),
		);
		if targets.is_empty() {
			succ!();
		}

		let mut favorites = cx.mgr.favorites.clone();
		let changed = match form.state {
			Some(state) => favorites.set_many(targets.iter(), state),
			None => favorites.toggle_many(targets.iter()),
		};
		if changed == 0 {
			succ!();
		}

		FavoritesIo::save(&favorites)?;
		cx.mgr.favorites = favorites;
		render!();
		succ!();
	}
}

fn resolve_targets<'a>(
	urls: &[UrlBuf],
	selected: impl Iterator<Item = &'a UrlBuf>,
	hovered: Option<&'a UrlBuf>,
) -> Vec<UrlBuf> {
	if !urls.is_empty() {
		return urls.to_vec();
	}

	let selected: Vec<_> = selected.cloned().collect();
	if !selected.is_empty() { selected } else { hovered.cloned().into_iter().collect() }
}

#[cfg(test)]
mod tests {
	use std::path::Path;

	use super::*;

	#[test]
	fn resolve_targets_prefers_explicit_urls() {
		let explicit: Vec<UrlBuf> = vec![Path::new("/explicit").into()];
		let selected: [UrlBuf; 1] = [Path::new("/selected").into()];
		let hovered: UrlBuf = Path::new("/hovered").into();

		assert_eq!(explicit, resolve_targets(&explicit, selected.iter(), Some(&hovered)),);
	}

	#[test]
	fn resolve_targets_uses_selected_before_hovered() {
		let selected: Vec<UrlBuf> =
			vec![Path::new("/selected-1").into(), Path::new("/selected-2").into()];
		let hovered: UrlBuf = Path::new("/hovered").into();

		assert_eq!(selected, resolve_targets(&[], selected.iter(), Some(&hovered)),);
	}

	#[test]
	fn resolve_targets_falls_back_to_hovered() {
		let hovered: UrlBuf = Path::new("/hovered").into();

		assert_eq!(vec![hovered.clone()], resolve_targets(&[], std::iter::empty(), Some(&hovered)),);
	}
}
