use mlua::AnyUserData;

use super::Lives;
use crate::lives::PtrCell;

#[derive(Clone, Copy)]
pub(super) struct Favorites;

impl Favorites {
	pub(super) fn make(inner: &yazi_core::mgr::Favorites) -> mlua::Result<AnyUserData> {
		let inner = PtrCell::from(inner);

		Lives::scoped_userdata(yazi_binding::Iter::new(
			inner.as_static().iter().map(yazi_binding::Url::new),
			Some(inner.len()),
		))
	}
}
