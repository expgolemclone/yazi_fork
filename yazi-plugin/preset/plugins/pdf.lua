local M = {}

function M:peek(job)
	local start, cache = os.clock(), self:cache(job)
	if not cache then
		return
	end

	local ok, err, bound = self:preload(job, cache)
	if bound and bound > 0 then
		return ya.emit("peek", { bound - 1, only_if = job.file.url, upper_bound = true })
	elseif not ok or err then
		return ya.preview_widget(job, err)
	end

	ya.sleep(math.max(0, rt.preview.image_delay / 1000 + start - os.clock()))

	local _, err = ya.image_show(cache, job.area)
	ya.preview_widget(job, err)
end

function M:seek(job)
	local h = cx.active.current.hovered
	if h and h.url == job.file.url then
		local step = ya.clamp(-1, job.units, 1)
		ya.emit("peek", { math.max(0, cx.active.preview.skip + step), only_if = job.file.url })
	end
end

function M:preload(job, cache)
	cache = cache or self:cache(job)
	if not cache or fs.cha(cache) then
		return true
	end

	local max_w, max_h = self:pixel_bounds(job)
	local scale = self:scale_args(job, max_w, max_h)
	local args = {
		"-f", job.skip + 1,
		"-l", job.skip + 1,
		"-singlefile",
	}
	for _, arg in ipairs(scale) do
		args[#args + 1] = arg
	end
	args[#args + 1] = "-jpeg"
	args[#args + 1] = "-jpegopt"
	args[#args + 1] = "quality=" .. rt.preview.image_quality
	args[#args + 1] = tostring(job.file.path)
	args[#args + 1] = tostring(cache)

	local output, err = Command("pdftoppm"):arg(args):output()

	if not output then
		return true, Err("Failed to start `pdftoppm`, error: %s", err)
	elseif not output.status.success then
		local pages = job.skip > 0 and tonumber(output.stderr:match("the last page %((%d+)%)"))
		return true, Err("Failed to convert PDF to image, stderr: %s", output.stderr), pages
	end

	return ya.image_precache(Url(cache .. ".jpg"), cache)
end

function M:cache(job)
	local base = ya.file_cache(job)
	if not base then
		return nil
	end

	local max_w, max_h = self:pixel_bounds(job)
	return Url(base .. string.format("-%dx%d", max_w, max_h))
end

function M:pixel_bounds(job)
	local max_w, max_h = rt.preview.max_width, rt.preview.max_height
	local cw, ch = rt.term.cell_size()
	if not cw or not ch then
		return max_w, max_h
	end

	local w = math.max(1, math.min(max_w, math.floor(job.area.w * cw)))
	local h = math.max(1, math.min(max_h, math.floor(job.area.h * ch)))
	return w, h
end

function M:scale_args(job, max_w, max_h)
	local page = self:page_aspect(job)
	if page and page < max_w / max_h then
		return { "-scale-to-x", -1, "-scale-to-y", max_h }
	else
		return { "-scale-to-x", max_w, "-scale-to-y", -1 }
	end
end

function M:page_aspect(job)
	local output = Command("pdfinfo")
		:arg({
			"-f", job.skip + 1,
			"-l", job.skip + 1,
			tostring(job.file.path),
		})
		:output()
	local result = output
	if not result or not result.status.success then
		return nil
	end

	local w, h = result.stdout:match("Page size:%s*([%d%.]+)%s+x%s+([%d%.]+)%s+pts")
	w, h = tonumber(w), tonumber(h)
	if not w or not h or w <= 0 or h <= 0 then
		return nil
	end
	return w / h
end

return M
