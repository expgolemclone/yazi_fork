use mlua::{ExternalError, FromLua, IntoLua, Lua, Value};
use yazi_shared::event::ActionCow;

#[derive(Debug)]
pub struct QuarterArrowForm {
	pub prev: bool,
}

impl From<ActionCow> for QuarterArrowForm {
	fn from(a: ActionCow) -> Self {
		Self { prev: a.bool("previous") }
	}
}

impl FromLua for QuarterArrowForm {
	fn from_lua(_: Value, _: &Lua) -> mlua::Result<Self> {
		Err("unsupported".into_lua_err())
	}
}

impl IntoLua for QuarterArrowForm {
	fn into_lua(self, _: &Lua) -> mlua::Result<Value> {
		Err("unsupported".into_lua_err())
	}
}
