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
		let Some(target) = cx.mgr.favorites.arrow(&current, form.prev).cloned() else {
			act!(notify:push, cx, MessageOpt {
				title:   "Favorite".to_owned(),
				content: "No favorites yet".to_owned(),
				level:   MessageLevel::Info,
				timeout: Duration::from_secs(3),
			})?;
			succ!();
		};

		act!(mgr:reveal, cx, RevealForm {
			target,
			raw: true,
			source: CdSource::Reveal,
			no_dummy: true,
		})
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

	use super::*;
	use crate::mgr::test_support::init_test_env;

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
