use std::time::Duration;

use anyhow::Result;
use indexmap::IndexSet;
use yazi_core::notify::{MessageLevel, MessageOpt};
use yazi_macro::{act, render, succ};
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
		let unique_targets: IndexSet<_> = targets.iter().cloned().collect();
		let before_on = unique_targets.iter().filter(|url| favorites.contains(url)).count();
		let changed = match form.state {
			Some(state) => favorites.set_many(targets.iter(), state),
			None => favorites.toggle_many(targets.iter()),
		};
		if changed == 0 {
			succ!();
		}
		let after_on = unique_targets.iter().filter(|url| favorites.contains(url)).count();
		let added = after_on.saturating_sub(before_on);
		let removed = before_on.saturating_sub(after_on);

		FavoritesIo::save(&favorites)?;
		cx.mgr.favorites = favorites;
		act!(notify:push, cx, MessageOpt {
			title:   "Favorite".to_owned(),
			content: favorite_message(unique_targets.len(), added, removed),
			level:   MessageLevel::Info,
			timeout: Duration::from_secs(3),
		})?;
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

fn favorite_message(total: usize, added: usize, removed: usize) -> String {
	match (total, added, removed) {
		(1, 1, 0) => "Added to favorites".to_owned(),
		(1, 0, 1) => "Removed from favorites".to_owned(),
		(_, a, 0) => format!("Added {a} item(s) to favorites"),
		(_, 0, r) => format!("Removed {r} item(s) from favorites"),
		(_, a, r) => format!("Updated favorites: {a} added, {r} removed"),
	}
}

#[cfg(test)]
mod tests {
	use std::{env, fs, path::{Path, PathBuf}, sync::Once, time::{SystemTime, UNIX_EPOCH}};

	use yazi_boot::BOOT;
	use yazi_core::Core;
	use yazi_fs::{File, FolderStage};
	use super::*;

	fn test_root() -> PathBuf { env::temp_dir().join("yazi-actor-favorite-tests") }

	fn init_test_env() {
		static ONCE: Once = Once::new();

		ONCE.call_once(|| {
			let root = test_root();
			fs::create_dir_all(root.join("config")).unwrap();
			fs::create_dir_all(root.join("state")).unwrap();

			unsafe {
				env::set_var("XDG_CONFIG_HOME", root.join("config"));
				env::set_var("XDG_STATE_HOME", root.join("state"));
			}

			yazi_shared::init_tests();
			yazi_fs::init_tests();
			yazi_config::init().unwrap();
			yazi_boot::init_default();
			yazi_watcher::init();
		});
	}

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

	#[test]
	fn favorite_message_describes_single_addition() {
		assert_eq!("Added to favorites", favorite_message(1, 1, 0));
	}

	#[test]
	fn favorite_message_describes_single_removal() {
		assert_eq!("Removed from favorites", favorite_message(1, 0, 1));
	}

	#[test]
	fn favorite_message_describes_mixed_update() {
		assert_eq!("Updated favorites: 2 added, 1 removed", favorite_message(3, 2, 1));
	}

	#[tokio::test(flavor = "current_thread")]
	async fn favorite_act_persists_and_notifies() {
		init_test_env();

		let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
		let file = PathBuf::from(format!("/tmp/yazi-favorite-{unique}.txt"));
		let cwd = file.parent().unwrap().to_path_buf();
		let state_file = BOOT.state_dir.join("favorites.json");

		let _ = fs::remove_file(&state_file);

		let mut core = Core::make();
		core.mgr.tabs[0].current.url = cwd.clone().into();
		core.mgr.tabs[0].current.files.update_full(vec![File::from_dummy(&file, None)]);
		core.mgr.tabs[0].current.stage = FolderStage::Loaded;
		core.mgr.tabs[0].current.cursor = 0;

		let mut term = None;
		let mut cx = Ctx::active(&mut core, &mut term);

		Favorite::act(&mut cx, FavoriteForm { urls: vec![file.clone().into()], state: None }).unwrap();

		assert!(cx.mgr.favorites.contains(&file));
		assert!(state_file.exists());

		let saved = fs::read_to_string(state_file).unwrap();
		assert!(saved.contains(&file.to_string_lossy().into_owned()));

		let msg = cx.notify.messages.last().unwrap();
		assert_eq!(msg.title, "Favorite");
		assert_eq!(msg.content, "Added to favorites");
	}
}
