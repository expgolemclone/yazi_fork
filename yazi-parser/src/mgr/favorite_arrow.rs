use mlua::{ExternalError, FromLua, IntoLua, Lua, Value};
use yazi_shared::event::ActionCow;

#[derive(Debug)]
pub struct FavoriteArrowForm {
	pub prev: bool,
}

impl From<ActionCow> for FavoriteArrowForm {
	fn from(a: ActionCow) -> Self {
		Self { prev: a.bool("previous") }
	}
}

impl FromLua for FavoriteArrowForm {
	fn from_lua(_: Value, _: &Lua) -> mlua::Result<Self> {
		Err("unsupported".into_lua_err())
	}
}

impl IntoLua for FavoriteArrowForm {
	fn into_lua(self, _: &Lua) -> mlua::Result<Value> {
		Err("unsupported".into_lua_err())
	}
}
