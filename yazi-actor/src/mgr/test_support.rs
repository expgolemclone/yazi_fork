#![cfg(test)]

use std::{env, fs, path::PathBuf, sync::Once};

pub(super) fn configured_favorites_file() -> PathBuf {
	test_root().join("managed/favorites.json")
}

pub(super) fn init_test_env() {
	static ONCE: Once = Once::new();

	ONCE.call_once(|| {
		let root = test_root();
		let _ = fs::remove_dir_all(&root);
		fs::create_dir_all(root.join("config/yazi")).unwrap();
		fs::create_dir_all(root.join("state")).unwrap();
		let favorites_file = configured_favorites_file().to_string_lossy().replace('\\', "\\\\");
		fs::write(
			root.join("config/yazi/yazi.toml"),
			format!("[mgr]\nfavorites_file = \"{favorites_file}\"\n"),
		)
		.unwrap();

		unsafe {
			env::set_var("XDG_CONFIG_HOME", root.join("config"));
			env::set_var("XDG_STATE_HOME", root.join("state"));
		}

		yazi_shared::init_tests();
		yazi_fs::init_tests();
		yazi_tty::init();
		yazi_term::init();
		yazi_config::init().unwrap();
		yazi_vfs::init();
		yazi_adapter::init().unwrap();
		yazi_boot::init_default();
		yazi_dds::init();
		yazi_widgets::init();
		yazi_watcher::init();
		yazi_runner::init(yazi_plugin::slim_lua);
		yazi_plugin::init().unwrap();
	});
}

fn test_root() -> PathBuf {
	env::temp_dir().join("yazi-actor-tests")
}
