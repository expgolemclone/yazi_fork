yazi_macro::mod_pub!(cha error mounts path provider);

yazi_macro::mod_flat!(cwd file files filter fns hash op scheme sorter sorting splatter stage url xdg);

pub fn init() {
	CWD.init(<_>::default());

	mounts::init();
}

pub fn init_tests() {
	static INIT: std::sync::OnceLock<()> = std::sync::OnceLock::new();

	INIT.get_or_init(init);
}
