Tab = {
	_id = "tab",
}

local function zero(area)
	return area { w = 0, h = 0 }
end

local function split_top(area, ratio)
	local bottom_total = ratio.parent + ratio.current
	local preview_raw, bottom = zero(area), zero(area)

	if ratio.preview == 0 then
		bottom = area
	elseif bottom_total == 0 then
		preview_raw = area
	else
		local rows = ui.Layout()
			:direction(ui.Layout.VERTICAL)
			:constraints({
				ui.Constraint.Ratio(ratio.preview, ratio.all),
				ui.Constraint.Ratio(bottom_total, ratio.all),
			})
			:split(area)
		preview_raw, bottom = rows[1], rows[2]
	end

	local parent_raw, current_raw = zero(bottom), zero(bottom)
	if bottom_total ~= 0 and bottom.w > 0 and bottom.h > 0 then
		if ratio.parent == 0 then
			current_raw = bottom
		elseif ratio.current == 0 then
			parent_raw = bottom
		else
			local cols = ui.Layout()
				:direction(ui.Layout.HORIZONTAL)
				:constraints({
					ui.Constraint.Ratio(ratio.parent, bottom_total),
					ui.Constraint.Ratio(ratio.current, bottom_total),
				})
				:split(bottom)
			parent_raw, current_raw = cols[1], cols[2]
		end
	end

	local top = preview_raw.h > 0 and bottom.h > 0 and 1 or 0
	return {
		position = "top",
		area = area,
		bottom = bottom,
		[1] = parent_raw,
		[2] = current_raw,
		[3] = preview_raw,
		parent = parent_raw:pad(ui.Pad(top, current_raw.w > 0 and 0 or 1, 0, 1)),
		current = current_raw:pad(ui.Pad(top, 1, 0, 1)),
		preview = preview_raw:pad(ui.Pad(0, 1, 0, 1)),
		marker_parent = parent_raw:pad(ui.Pad(top, 0, 0, 0)),
		marker_current = current_raw:pad(ui.Pad(top, 0, 0, 0)),
	}
end

local function split_right(area, ratio)
	local cols = ui.Layout()
		:direction(ui.Layout.HORIZONTAL)
		:constraints({
			ui.Constraint.Ratio(ratio.parent, ratio.all),
			ui.Constraint.Ratio(ratio.current, ratio.all),
			ui.Constraint.Ratio(ratio.preview, ratio.all),
		})
		:split(area)

	local p = cols[2].w > 0 and 0 or 1
	return {
		position = "right",
		area = area,
		[1] = cols[1],
		[2] = cols[2],
		[3] = cols[3],
		parent = cols[1]:pad(ui.Pad(0, p, 0, 1)),
		current = cols[2]:pad(ui.Pad.x(1)),
		preview = cols[3]:pad(ui.Pad(0, 1, 0, p)),
		marker_parent = cols[1],
		marker_current = cols[2],
	}
end

function Tab:new(area, tab)
	local me = setmetatable({ _area = area, _tab = tab }, { __index = self })
	me:layout()
	me:build()
	return me
end

function Tab:layout()
	local ratio = rt.mgr.ratio
	self._chunks = rt.preview.position == "top" and split_top(self._area, ratio)
		or split_right(self._area, ratio)
end

function Tab:build()
	local c = self._chunks
	self._children = {
		Parent:new(c.parent, self._tab),
		Current:new(c.current, self._tab),
		Preview:new(c.preview, self._tab),
		Rails:new(c, self._tab),
		Markers:new(c, self._tab),
	}
end

function Tab:reflow()
	local components = { self }
	for _, child in ipairs(self._children) do
		components = ya.list_merge(components, child:reflow())
	end
	return components
end

function Tab:redraw()
	local elements = self._base or {}
	for _, child in ipairs(self._children) do
		elements = ya.list_merge(elements, ui.redraw(child))
	end
	return elements
end

-- Mouse events
function Tab:click(event, up) end

function Tab:scroll(event, step) end

function Tab:touch(event, step) end
