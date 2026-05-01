use std::{
	ffi::OsString,
	fs,
	path::{Path, PathBuf},
	time::Duration,
};

use anyhow::{Context, Result};
use yazi_core::{
	mgr::CdSource,
	notify::{MessageLevel, MessageOpt},
};
use yazi_macro::{act, succ};
use yazi_parser::mgr::{QuarterArrowForm, RevealForm};
use yazi_shared::{data::Data, url::UrlBuf};

use crate::{Actor, Ctx};

pub struct QuarterArrow;

impl Actor for QuarterArrow {
	type Form = QuarterArrowForm;

	const NAME: &str = "quarter_arrow";

	fn act(cx: &mut Ctx, form: Self::Form) -> Result<Data> {
		let Some(current) = cx.hovered().map(|hovered| hovered.url.clone()) else {
			succ!();
		};
		let Some(path) = current.clone().into_local() else {
			succ!();
		};
		let preserving_cycle = cx.mgr.favorite_cycle.matches(&current);

		match resolve_target(&path, form.prev)? {
			Target::Ignore => succ!(),
			Target::Notify(content) => {
				notify(cx, content)?;
				succ!();
			}
			Target::Reveal(target) => {
				let data = act!(mgr:reveal, cx, RevealForm {
					target,
					raw: true,
					source: CdSource::Reveal,
					no_dummy: true,
				})?;
				if preserving_cycle {
					let landed = current_target(cx);
					cx.mgr.favorite_cycle.relocate(landed);
				}
				Ok(data)
			}
		}
	}
}

fn current_target(cx: &Ctx) -> UrlBuf {
	cx.hovered().map(|hovered| hovered.url.clone()).unwrap_or_else(|| cx.cwd().clone())
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct QuarterKey {
	year: u16,
	quarter: u8,
}

struct QuarterPath {
	data_root: PathBuf,
	current: QuarterKey,
	filename: OsString,
}

enum Target {
	Ignore,
	Notify(&'static str),
	Reveal(UrlBuf),
}

fn resolve_target(path: &Path, prev: bool) -> Result<Target> {
	let Some(current) = parse_quarter_path(path) else {
		return Ok(Target::Ignore);
	};

	let quarters = quarter_dirs(&current.data_root).with_context(|| {
		format!("failed to read quarter directories under {}", current.data_root.display())
	})?;
	let Some(index) = quarters.iter().position(|(key, _)| *key == current.current) else {
		return Ok(Target::Ignore);
	};

	let Some((_, dir)) = prev_index(index, quarters.len(), prev).and_then(|i| quarters.get(i)) else {
		return Ok(Target::Notify(if prev { "No previous quarter" } else { "No next quarter" }));
	};

	let target = dir.join(&current.filename);
	if target.is_file() {
		Ok(Target::Reveal(target.into()))
	} else {
		Ok(Target::Notify("No quarterly PDF for this ticker"))
	}
}

fn parse_quarter_path(path: &Path) -> Option<QuarterPath> {
	if !path.extension()?.to_str()?.eq_ignore_ascii_case("pdf") {
		return None;
	}

	let filename = path.file_name()?.to_os_string();
	let quarter_dir = path.parent()?;
	let data_root = quarter_dir.parent()?;
	let current = parse_quarter_dir_name(quarter_dir.file_name()?.to_str()?)?;

	Some(QuarterPath { data_root: data_root.to_path_buf(), current, filename })
}

fn parse_quarter_dir_name(name: &str) -> Option<QuarterKey> {
	let (year, quarter) = name.split_once('_')?;
	if year.len() != 4 || !year.bytes().all(|b| b.is_ascii_digit()) || quarter.len() != 1 {
		return None;
	}

	let quarter = quarter.parse().ok()?;
	if !(1..=4).contains(&quarter) {
		return None;
	}

	Some(QuarterKey { year: year.parse().ok()?, quarter })
}

fn quarter_dirs(data_root: &Path) -> std::io::Result<Vec<(QuarterKey, PathBuf)>> {
	let mut quarters = vec![];

	for entry in fs::read_dir(data_root)? {
		let entry = entry?;
		if !entry.file_type().is_ok_and(|ft| ft.is_dir()) {
			continue;
		}

		let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
			continue;
		};
		let Some(key) = parse_quarter_dir_name(&name) else {
			continue;
		};
		quarters.push((key, entry.path()));
	}

	quarters.sort_by_key(|(key, _)| *key);
	Ok(quarters)
}

fn prev_index(current: usize, len: usize, prev: bool) -> Option<usize> {
	if prev {
		current.checked_sub(1)
	} else if current + 1 < len {
		Some(current + 1)
	} else {
		None
	}
}

fn notify(cx: &mut Ctx, content: &str) -> Result<()> {
	act!(notify:push, cx, MessageOpt {
		title:   "Quarter".to_owned(),
		content: content.to_owned(),
		level:   MessageLevel::Info,
		timeout: Duration::from_secs(3),
	})?;
	Ok(())
}

#[cfg(test)]
mod tests {
	use std::{
		env, fs,
		path::{Path, PathBuf},
		time::{SystemTime, UNIX_EPOCH},
	};

	use yazi_core::{Core, tab::Folder};
	use yazi_fs::{File, FolderStage};
	use yazi_parser::mgr::FavoriteArrowForm;
	use yazi_shared::url::UrlBuf;

	use super::*;
	use crate::mgr::{FavoriteArrow, test_support::init_test_env};

	fn loaded_folder(dir: &Path, files: Vec<File>, cursor: usize) -> Folder {
		let mut folder = Folder::from(dir.to_path_buf());
		folder.files.update_full(files);
		folder.stage = FolderStage::Loaded;
		folder.cursor = cursor;
		folder
	}

	fn unique_root(name: &str) -> PathBuf {
		let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
		env::temp_dir().join(format!("yazi-quarter-arrow-{name}-{unique}"))
	}

	fn create_pdf(path: &Path) {
		fs::create_dir_all(path.parent().unwrap()).unwrap();
		fs::write(path, b"%PDF-1.7").unwrap();
	}

	#[tokio::test(flavor = "current_thread")]
	async fn quarter_arrow_act_reveals_previous_quarter_pdf() {
		init_test_env();

		let root = unique_root("previous");
		let current = root.join("data/2026_2/1301.pdf");
		let target = root.join("data/2026_1/1301.pdf");
		create_pdf(&current);
		create_pdf(&target);

		let current_dir = current.parent().unwrap();
		let target_dir = target.parent().unwrap();

		let mut core = Core::make();
		core.mgr.tabs[0].current =
			loaded_folder(current_dir, vec![File::from_dummy(&current, None)], 0);
		core.mgr.tabs[0].history.insert(
			target_dir.to_path_buf().into(),
			loaded_folder(target_dir, vec![File::from_dummy(&target, None)], 0),
		);

		let mut term = None;
		let mut cx = Ctx::active(&mut core, &mut term);

		QuarterArrow::act(&mut cx, QuarterArrowForm { prev: true }).unwrap();

		assert_eq!(&UrlBuf::from(target_dir), cx.cwd());
		assert_eq!(Some(&UrlBuf::from(target)), cx.hovered().map(|hovered| &hovered.url));
	}

	#[tokio::test(flavor = "current_thread")]
	async fn quarter_arrow_act_reveals_next_quarter_pdf() {
		init_test_env();

		let root = unique_root("next");
		let current = root.join("data/2026_1/1301.pdf");
		let target = root.join("data/2026_2/1301.pdf");
		create_pdf(&current);
		create_pdf(&target);

		let current_dir = current.parent().unwrap();
		let target_dir = target.parent().unwrap();

		let mut core = Core::make();
		core.mgr.tabs[0].current =
			loaded_folder(current_dir, vec![File::from_dummy(&current, None)], 0);
		core.mgr.tabs[0].history.insert(
			target_dir.to_path_buf().into(),
			loaded_folder(target_dir, vec![File::from_dummy(&target, None)], 0),
		);

		let mut term = None;
		let mut cx = Ctx::active(&mut core, &mut term);

		QuarterArrow::act(&mut cx, QuarterArrowForm { prev: false }).unwrap();

		assert_eq!(&UrlBuf::from(target_dir), cx.cwd());
		assert_eq!(Some(&UrlBuf::from(target)), cx.hovered().map(|hovered| &hovered.url));
	}

	#[tokio::test(flavor = "current_thread")]
	async fn quarter_arrow_act_skips_missing_quarter_numbers() {
		init_test_env();

		let root = unique_root("gap");
		let current = root.join("data/2026_1/1301.pdf");
		let target = root.join("data/2026_3/1301.pdf");
		create_pdf(&current);
		create_pdf(&target);

		let current_dir = current.parent().unwrap();
		let target_dir = target.parent().unwrap();

		let mut core = Core::make();
		core.mgr.tabs[0].current =
			loaded_folder(current_dir, vec![File::from_dummy(&current, None)], 0);
		core.mgr.tabs[0].history.insert(
			target_dir.to_path_buf().into(),
			loaded_folder(target_dir, vec![File::from_dummy(&target, None)], 0),
		);

		let mut term = None;
		let mut cx = Ctx::active(&mut core, &mut term);

		QuarterArrow::act(&mut cx, QuarterArrowForm { prev: false }).unwrap();

		assert_eq!(Some(&UrlBuf::from(target)), cx.hovered().map(|hovered| &hovered.url));
	}

	#[tokio::test(flavor = "current_thread")]
	async fn quarter_arrow_act_keeps_favorite_cycle_for_next_favorite() {
		init_test_env();

		let root = unique_root("favorite-cycle-next");
		let quarter_1 = root.join("data/2026_1");
		let quarter_2 = root.join("data/2026_2");
		let a = quarter_1.join("1301.pdf");
		let b = quarter_1.join("2802.pdf");
		let c = quarter_1.join("7203.pdf");
		let b_next = quarter_2.join("2802.pdf");
		create_pdf(&a);
		create_pdf(&b);
		create_pdf(&c);
		create_pdf(&b_next);

		let mut core = Core::make();
		core.mgr.favorites =
			[a.clone().into(), b.clone().into(), c.clone().into()].into_iter().collect();
		core.mgr.tabs[0].current = loaded_folder(
			&quarter_1,
			vec![File::from_dummy(&a, None), File::from_dummy(&b, None), File::from_dummy(&c, None)],
			0,
		);
		core.mgr.tabs[0].history.insert(
			quarter_2.clone().into(),
			loaded_folder(&quarter_2, vec![File::from_dummy(&b_next, None)], 0),
		);

		let mut term = None;
		let mut cx = Ctx::active(&mut core, &mut term);

		FavoriteArrow::act(&mut cx, FavoriteArrowForm { prev: false }).unwrap();
		assert_eq!(Some(&UrlBuf::from(b.clone())), cx.hovered().map(|hovered| &hovered.url));

		QuarterArrow::act(&mut cx, QuarterArrowForm { prev: false }).unwrap();
		assert_eq!(&UrlBuf::from(quarter_2.clone()), cx.cwd());
		assert_eq!(Some(&UrlBuf::from(b_next)), cx.hovered().map(|hovered| &hovered.url));

		FavoriteArrow::act(&mut cx, FavoriteArrowForm { prev: false }).unwrap();
		assert_eq!(&UrlBuf::from(quarter_1), cx.cwd());
		assert_eq!(Some(&UrlBuf::from(c)), cx.hovered().map(|hovered| &hovered.url));
	}

	#[tokio::test(flavor = "current_thread")]
	async fn quarter_arrow_act_keeps_favorite_cycle_for_previous_favorite() {
		init_test_env();

		let root = unique_root("favorite-cycle-previous");
		let quarter_1 = root.join("data/2026_1");
		let quarter_2 = root.join("data/2026_2");
		let a = quarter_1.join("1301.pdf");
		let b = quarter_1.join("2802.pdf");
		let c = quarter_1.join("7203.pdf");
		let b_next = quarter_2.join("2802.pdf");
		create_pdf(&a);
		create_pdf(&b);
		create_pdf(&c);
		create_pdf(&b_next);

		let mut core = Core::make();
		core.mgr.favorites =
			[a.clone().into(), b.clone().into(), c.clone().into()].into_iter().collect();
		core.mgr.tabs[0].current = loaded_folder(
			&quarter_1,
			vec![File::from_dummy(&a, None), File::from_dummy(&b, None), File::from_dummy(&c, None)],
			0,
		);
		core.mgr.tabs[0].history.insert(
			quarter_2.clone().into(),
			loaded_folder(&quarter_2, vec![File::from_dummy(&b_next, None)], 0),
		);

		let mut term = None;
		let mut cx = Ctx::active(&mut core, &mut term);

		FavoriteArrow::act(&mut cx, FavoriteArrowForm { prev: false }).unwrap();
		assert_eq!(Some(&UrlBuf::from(b.clone())), cx.hovered().map(|hovered| &hovered.url));

		QuarterArrow::act(&mut cx, QuarterArrowForm { prev: false }).unwrap();
		assert_eq!(&UrlBuf::from(quarter_2.clone()), cx.cwd());
		assert_eq!(Some(&UrlBuf::from(b_next)), cx.hovered().map(|hovered| &hovered.url));

		FavoriteArrow::act(&mut cx, FavoriteArrowForm { prev: true }).unwrap();
		assert_eq!(&UrlBuf::from(quarter_1), cx.cwd());
		assert_eq!(Some(&UrlBuf::from(a)), cx.hovered().map(|hovered| &hovered.url));
	}

	#[tokio::test(flavor = "current_thread")]
	async fn quarter_arrow_act_notifies_when_target_pdf_is_missing() {
		init_test_env();

		let root = unique_root("missing-pdf");
		let current = root.join("data/2026_2/1301.pdf");
		create_pdf(&current);
		fs::create_dir_all(root.join("data/2026_1")).unwrap();

		let current_dir = current.parent().unwrap();

		let mut core = Core::make();
		core.mgr.tabs[0].current =
			loaded_folder(current_dir, vec![File::from_dummy(&current, None)], 0);

		let mut term = None;
		let mut cx = Ctx::active(&mut core, &mut term);

		QuarterArrow::act(&mut cx, QuarterArrowForm { prev: true }).unwrap();

		let msg = cx.notify.messages.last().unwrap();
		assert_eq!(msg.title, "Quarter");
		assert_eq!(msg.content, "No quarterly PDF for this ticker");
		assert_eq!(Some(&UrlBuf::from(current)), cx.hovered().map(|hovered| &hovered.url));
	}

	#[tokio::test(flavor = "current_thread")]
	async fn quarter_arrow_act_ignores_non_quarter_paths() {
		init_test_env();

		let root = unique_root("ignore");
		let current = root.join("misc/1301.pdf");
		create_pdf(&current);

		let current_dir = current.parent().unwrap();

		let mut core = Core::make();
		core.mgr.tabs[0].current =
			loaded_folder(current_dir, vec![File::from_dummy(&current, None)], 0);

		let mut term = None;
		let mut cx = Ctx::active(&mut core, &mut term);

		QuarterArrow::act(&mut cx, QuarterArrowForm { prev: true }).unwrap();

		assert!(cx.notify.messages.is_empty());
		assert_eq!(&UrlBuf::from(current_dir), cx.cwd());
		assert_eq!(Some(&UrlBuf::from(current)), cx.hovered().map(|hovered| &hovered.url));
	}
}
