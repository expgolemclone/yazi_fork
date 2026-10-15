Rail = {}

function Rail:new(id, area, chunks)
	return setmetatable({
		_id = id,
		_area = area,
		_chunks = chunks,
	}, { __index = self })
end

function Rail:reflow() return { self } end

function Rail:redraw()
	return {
		ui.Bar(self._id == "rail-top" and ui.Edge.TOP or ui.Edge.LEFT)
			:area(self._area)
			:symbol(th.mgr.border_symbol)
			:style(th.mgr.border_style),
	}
end

-- Mouse events
function Rail:click(event, up) end

function Rail:scroll(event, step) end

function Rail:touch(event, step) end

function Rail:drag(event)
	local c, x, y, parent, current, preview = self._chunks, 0, 0, 0, 0, 0
	if c.position == "top" and self._id == "rail-left" then
		local sum = rt.mgr.ratio.parent + rt.mgr.ratio.current
		if sum <= 1 or c.bottom.w <= 1 then
			return
		end

		x = ya.clamp(c.bottom.x + 1, event.x, c.bottom.right - 1)
		parent = ya.clamp(1, math.floor(((x - c.bottom.x) * sum / c.bottom.w) + 0.5), sum - 1)
		current = sum - parent
		preview = rt.mgr.ratio.preview
	elseif c.position == "top" and self._id == "rail-top" then
		local sum = rt.mgr.ratio.parent + rt.mgr.ratio.current
		if sum == 0 or c.area.h <= 1 then
			return
		end

		y = ya.clamp(c.area.y + 1, event.y, c.area.bottom - 1)
		preview = math.max(1, math.floor((sum * (y - c.area.y) / (c.area.bottom - y)) + 0.5))
		parent = rt.mgr.ratio.parent
		current = rt.mgr.ratio.current
	elseif self._id == "rail-left" then
		x = math.min(event.x, c[2].right - 2)
		parent = math.max(1, x - c[1].x)
		current = math.max(1, c[1].w + c[2].w - parent)
		preview = math.max(1, c[3].w)
	else
		x = math.max(event.x, c[2].x + 2)
		preview = math.max(1, c[3].right - x)
		current = math.max(1, c[2].w + c[3].w - preview)
		parent = math.max(1, c[1].w)
	end

	local r = rt.mgr.ratio
	if r.parent ~= parent or r.current ~= current or r.preview ~= preview then
		rt.mgr.ratio = { parent, current, preview }
		ui.render()
	end
end
