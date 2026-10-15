use anyhow::Result;
use yazi_fs::{File, FilesOp};
use yazi_macro::{act, render, succ};
use yazi_parser::mgr::RevealForm;
use yazi_shared::{data::Data, path::AsPath, url::UrlLike};

use crate::{Actor, Ctx};

pub struct Reveal;

impl Actor for Reveal {
	type Form = RevealForm;

	const NAME: &str = "reveal";

	fn act(cx: &mut Ctx, form: Self::Form) -> Result<Data> {
		let Some((parent, child)) = form.target.pair() else { succ!() };
		let child = child.to_owned();

		// Cd to the parent directory
		act!(mgr:cd, cx, (parent, form.source))?;

		// Try to hover over the child file
		let tab = cx.tab_mut();
		render!(tab.current.hover(child.as_path()));

		// If the child is not hovered, which means it doesn't exist,
		// create a dummy file
		if !form.no_dummy && tab.hovered().is_none_or(|f| child.as_path() != f.urn()) {
			let op = FilesOp::Creating(parent.into(), vec![File::from_dummy(&form.target, None)]);
			tab.current.update_pub(tab.id, op);
		}

		// Now, we can safely hover over the target
		act!(mgr:hover, cx, Some(child.clone()))?;
		if form.no_dummy {
			// Keep tracking the requested child until the directory contents finish loading.
			cx.current_mut().trace = Some(child);
		}

		act!(mgr:peek, cx)?;
		act!(mgr:watch, cx)?;
		succ!();
	}
}

#[cfg(test)]
mod tests {
	use std::{
		env, fs,
		path::{Path, PathBuf},
		time::{SystemTime, UNIX_EPOCH},
	};

	use yazi_core::{Core, mgr::CdSource, tab::Folder};
	use yazi_fs::{File, FilesOp, FolderStage};
	use yazi_parser::mgr::UpdateFilesForm;
	use yazi_shared::url::UrlBuf;

	use super::*;
	use crate::mgr::{UpdateFiles, test_support::init_test_env};

	fn loaded_folder(dir: &Path, files: Vec<File>, cursor: usize) -> Folder {
		let mut folder = Folder::from(dir.to_path_buf());
		folder.files.update_full(files);
		folder.stage = FolderStage::Loaded;
		folder.cursor = cursor;
		folder
	}

	fn unique_root(name: &str) -> PathBuf {
		let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
		env::temp_dir().join(format!("yazi-reveal-{name}-{unique}"))
	}

	fn create_file(path: &Path) {
		fs::create_dir_all(path.parent().unwrap()).unwrap();
		fs::write(path, b"dummy").unwrap();
	}

	#[tokio::test(flavor = "current_thread")]
	async fn reveal_act_keeps_no_dummy_target_hovered_after_directory_load() {
		init_test_env();

		let root = unique_root("no-dummy-trace");
		let current_dir = root.join("data/2026_1");
		let target_dir = root.join("data/2026_2");
		let current = current_dir.join("2802.pdf");
		let target = target_dir.join("2802.pdf");
		let other = target_dir.join("1301.pdf");
		create_file(&current);
		create_file(&target);
		create_file(&other);

		let mut core = Core::make();
		core.mgr.tabs[0].current =
			loaded_folder(&current_dir, vec![File::from_dummy(&current, None)], 0);

		let mut term = None;
		let mut cx = Ctx::active(&mut core, &mut term);

		Reveal::act(
			&mut cx,
			RevealForm {
				target: target.clone().into(),
				raw: true,
				source: CdSource::Reveal,
				no_dummy: true,
			},
		)
		.unwrap();

		assert_eq!(&UrlBuf::from(target_dir.clone()), cx.cwd());
		assert!(cx.hovered().is_none());

		UpdateFiles::act(
			&mut cx,
			UpdateFilesForm::from(FilesOp::Full(
				target_dir.clone().into(),
				vec![File::from_dummy(&other, None), File::from_dummy(&target, None)],
				Default::default(),
			)),
		)
		.unwrap();

		assert_eq!(Some(&UrlBuf::from(target)), cx.hovered().map(|hovered| &hovered.url));
	}
}
