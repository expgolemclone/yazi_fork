use mlua::{ExternalError, FromLua, IntoLua, Lua, Value};
use yazi_shared::{event::ActionCow, url::UrlBuf};

#[derive(Debug)]
pub struct FavoriteForm {
	pub urls: Vec<UrlBuf>,
	pub state: Option<bool>,
}

impl From<ActionCow> for FavoriteForm {
	fn from(mut a: ActionCow) -> Self {
		Self {
			urls: a.take_seq(),
			state: match a.get("state") {
				Ok("on") => Some(true),
				Ok("off") => Some(false),
				_ => None,
			},
		}
	}
}

impl FromLua for FavoriteForm {
	fn from_lua(_: Value, _: &Lua) -> mlua::Result<Self> {
		Err("unsupported".into_lua_err())
	}
}

impl IntoLua for FavoriteForm {
	fn into_lua(self, _: &Lua) -> mlua::Result<Value> {
		Err("unsupported".into_lua_err())
	}
}
