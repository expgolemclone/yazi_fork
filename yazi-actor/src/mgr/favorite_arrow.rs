use std::time::Duration;

use anyhow::Result;
use yazi_core::{
	mgr::CdSource,
	notify::{MessageLevel, MessageOpt},
};
use yazi_macro::{act, succ};
use yazi_parser::mgr::{FavoriteArrowForm, RevealForm};
use yazi_shared::{data::Data, url::UrlBuf};

use crate::{Actor, Ctx};

pub struct FavoriteArrow;

impl Actor for FavoriteArrow {
	type Form = FavoriteArrowForm;

	const NAME: &str = "favorite_arrow";

	fn act(cx: &mut Ctx, form: Self::Form) -> Result<Data> {
		let current = current_target(cx);
		let target = if cx.mgr.favorite_cycle.matches(&current) {
			cx.mgr
				.favorite_cycle
				.target(form.prev)
				.cloned()
				.or_else(|| cx.mgr.favorites.arrow(&current, form.prev).cloned())
		} else {
			cx.mgr.favorites.arrow(&current, form.prev).cloned()
		};
		let Some(target) = target else {
			act!(notify:push, cx, MessageOpt {
				title:   "Favorite".to_owned(),
				content: "No favorites yet".to_owned(),
				level:   MessageLevel::Info,
				timeout: Duration::from_secs(3),
			})?;
			succ!();
		};

		let data = act!(mgr:reveal, cx, RevealForm {
			target,
			raw: true,
			source: CdSource::Reveal,
			no_dummy: true,
		})?;
		let landed = current_target(cx);
		let favorites = cx.mgr.favorites.clone();
		cx.mgr.favorite_cycle.advance(&favorites, landed);
		Ok(data)
	}
}

fn current_target(cx: &Ctx) -> UrlBuf {
	cx.hovered().map(|hovered| hovered.url.clone()).unwrap_or_else(|| cx.cwd().clone())
}

#[cfg(test)]
mod tests {
	use std::path::{Path, PathBuf};

	use yazi_boot::BOOT;
	use yazi_core::{Core, tab::Folder};
	use yazi_fs::{File, FolderStage};
	use yazi_shared::path::PathBufDyn;

	use super::*;
	use crate::mgr::{Hover, test_support::init_test_env};

	fn loaded_folder(dir: &Path, files: Vec<File>, cursor: usize) -> Folder {
		let mut folder = Folder::from(dir.to_path_buf());
		folder.files.update_full(files);
		folder.stage = FolderStage::Loaded;
		folder.cursor = cursor;
		folder
	}

	#[tokio::test(flavor = "current_thread")]
	async fn current_target_prefers_hovered_file() {
		init_test_env();

		let dir = PathBuf::from("/tmp/favorite-arrow-current");
		let hovered = dir.join("hovered.txt");
		let mut core = Core::make();
		core.mgr.tabs[0].current = loaded_folder(&dir, vec![File::from_dummy(&hovered, None)], 0);

		let mut term = None;
		let cx = Ctx::active(&mut core, &mut term);

		assert_eq!(UrlBuf::from(hovered), current_target(&cx));
	}

	#[tokio::test(flavor = "current_thread")]
	async fn favorite_arrow_act_reveals_next_favorite_in_same_directory() {
		init_test_env();

		let dir = PathBuf::from("/tmp/yazi-favorite-arrow-same");
		let first = dir.join("a.txt");
		let second = dir.join("b.txt");

		let mut core = Core::make();
		core.mgr.favorites = [first.clone().into(), second.clone().into()].into_iter().collect();
		core.mgr.tabs[0].current =
			loaded_folder(&dir, vec![File::from_dummy(&first, None), File::from_dummy(&second, None)], 0);

		let mut term = None;
		let mut cx = Ctx::active(&mut core, &mut term);

		FavoriteArrow::act(&mut cx, FavoriteArrowForm { prev: false }).unwrap();

		assert_eq!(&UrlBuf::from(dir), cx.cwd());
		assert_eq!(Some(&UrlBuf::from(second)), cx.hovered().map(|hovered| &hovered.url));
	}

	#[tokio::test(flavor = "current_thread")]
	async fn favorite_arrow_act_reveals_saved_favorite_in_another_directory() {
		init_test_env();

		let current_dir = PathBuf::from("/tmp/yazi-favorite-arrow-current");
		let other_dir = PathBuf::from("/tmp/yazi-favorite-arrow-other");
		let current = current_dir.join("current.txt");
		let target = other_dir.join("target.txt");

		let mut core = Core::make();
		core.mgr.favorites = [target.clone().into()].into_iter().collect();
		core.mgr.tabs[0].current =
			loaded_folder(&current_dir, vec![File::from_dummy(&current, None)], 0);
		core.mgr.tabs[0].history.insert(
			other_dir.clone().into(),
			loaded_folder(&other_dir, vec![File::from_dummy(&target, None)], 0),
		);

		let mut term = None;
		let mut cx = Ctx::active(&mut core, &mut term);

		FavoriteArrow::act(&mut cx, FavoriteArrowForm { prev: false }).unwrap();

		assert_eq!(&UrlBuf::from(other_dir), cx.cwd());
		assert_eq!(Some(&UrlBuf::from(target)), cx.hovered().map(|hovered| &hovered.url));
	}

	#[tokio::test(flavor = "current_thread")]
	async fn favorite_arrow_act_wraps_backward_from_non_favorite_to_last() {
		init_test_env();

		let dir = PathBuf::from("/tmp/yazi-favorite-arrow-wrap");
		let other = dir.join("other.txt");
		let first = dir.join("first.txt");
		let last = dir.join("last.txt");

		let mut core = Core::make();
		core.mgr.favorites = [first.clone().into(), last.clone().into()].into_iter().collect();
		core.mgr.tabs[0].current = loaded_folder(
			&dir,
			vec![
				File::from_dummy(&other, None),
				File::from_dummy(&first, None),
				File::from_dummy(&last, None),
			],
			0,
		);

		let mut term = None;
		let mut cx = Ctx::active(&mut core, &mut term);

		FavoriteArrow::act(&mut cx, FavoriteArrowForm { prev: true }).unwrap();

		assert_eq!(Some(&UrlBuf::from(last)), cx.hovered().map(|hovered| &hovered.url));
	}

	#[tokio::test(flavor = "current_thread")]
	async fn favorite_arrow_act_keeps_cycle_after_current_favorite_is_removed() {
		init_test_env();

		let dir = PathBuf::from("/tmp/yazi-favorite-arrow-cycle");
		let a = dir.join("a.txt");
		let b = dir.join("b.txt");
		let c = dir.join("c.txt");
		let d = dir.join("d.txt");

		let mut core = Core::make();
		core.mgr.favorites = [a.clone().into(), b.clone().into(), c.clone().into(), d.clone().into()]
			.into_iter()
			.collect();
		core.mgr.tabs[0].current = loaded_folder(
			&dir,
			vec![
				File::from_dummy(&a, None),
				File::from_dummy(&b, None),
				File::from_dummy(&c, None),
				File::from_dummy(&d, None),
			],
			0,
		);

		let mut term = None;
		let mut cx = Ctx::active(&mut core, &mut term);

		FavoriteArrow::act(&mut cx, FavoriteArrowForm { prev: false }).unwrap();
		FavoriteArrow::act(&mut cx, FavoriteArrowForm { prev: false }).unwrap();
		assert_eq!(Some(&UrlBuf::from(c.clone())), cx.hovered().map(|hovered| &hovered.url));

		let before = cx.mgr.favorites.clone();
		let mut after = before.clone();
		assert_eq!(1, after.toggle_many([c.as_path()]));
		cx.mgr.favorite_cycle.reconcile(&before, &after);
		cx.mgr.favorites = after;
		assert!(!cx.mgr.favorites.contains(&c));

		FavoriteArrow::act(&mut cx, FavoriteArrowForm { prev: false }).unwrap();
		assert_eq!(Some(&UrlBuf::from(d)), cx.hovered().map(|hovered| &hovered.url));

		FavoriteArrow::act(&mut cx, FavoriteArrowForm { prev: true }).unwrap();
		assert_eq!(Some(&UrlBuf::from(b)), cx.hovered().map(|hovered| &hovered.url));
	}

	#[tokio::test(flavor = "current_thread")]
	async fn favorite_arrow_act_falls_back_to_hovered_file_after_manual_move() {
		init_test_env();

		let dir = PathBuf::from("/tmp/yazi-favorite-arrow-manual");
		let a = dir.join("a.txt");
		let b = dir.join("b.txt");
		let c = dir.join("c.txt");
		let x = dir.join("x.txt");

		let mut core = Core::make();
		core.mgr.favorites =
			[a.clone().into(), b.clone().into(), c.clone().into()].into_iter().collect();
		core.mgr.tabs[0].current = loaded_folder(
			&dir,
			vec![
				File::from_dummy(&a, None),
				File::from_dummy(&b, None),
				File::from_dummy(&c, None),
				File::from_dummy(&x, None),
			],
			0,
		);

		let mut term = None;
		let mut cx = Ctx::active(&mut core, &mut term);

		FavoriteArrow::act(&mut cx, FavoriteArrowForm { prev: false }).unwrap();
		assert_eq!(Some(&UrlBuf::from(b.clone())), cx.hovered().map(|hovered| &hovered.url));

		Hover::act(&mut cx, Some(PathBufDyn::from(Path::new("x.txt"))).into()).unwrap();
		assert_eq!(Some(&UrlBuf::from(x.clone())), cx.hovered().map(|hovered| &hovered.url));

		FavoriteArrow::act(&mut cx, FavoriteArrowForm { prev: false }).unwrap();
		assert_eq!(Some(&UrlBuf::from(a)), cx.hovered().map(|hovered| &hovered.url));
	}

	#[tokio::test(flavor = "current_thread")]
	async fn favorite_arrow_act_notifies_when_empty() {
		init_test_env();

		let dir = BOOT.state_dir.join("favorite-arrow-empty");
		let file = dir.join("current.txt");

		let mut core = Core::make();
		core.mgr.tabs[0].current = loaded_folder(&dir, vec![File::from_dummy(&file, None)], 0);

		let mut term = None;
		let mut cx = Ctx::active(&mut core, &mut term);

		FavoriteArrow::act(&mut cx, FavoriteArrowForm { prev: false }).unwrap();

		let msg = cx.notify.messages.last().unwrap();
		assert_eq!(msg.title, "Favorite");
		assert_eq!(msg.content, "No favorites yet");
		assert_eq!(Some(&UrlBuf::from(file)), cx.hovered().map(|hovered| &hovered.url));
	}
}
