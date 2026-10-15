local M = {}

function M:peek(job)
	local start, cache = os.clock(), self:cache(job)
	if not cache then
		return
	end

	local ok, err = self:preload(job, cache)
	if not ok or err then
		return ya.preview_widget(job, err)
	end

	ya.sleep(math.max(0, rt.preview.image_delay / 1000 + start - os.clock()))

	local _, show_err = ya.image_show(cache, job.area)
	ya.preview_widget(job, show_err)
end

function M:seek() end

function M:preload(job, cache)
	cache = cache or self:cache(job)
	if not cache or fs.cha(cache) then
		return true
	end

	local source = self:source_cache(job)
	local ok, err = self:rasterize(job, source)
	if not ok or err then
		return ok, err
	end

	local alpha, alpha_err = self:has_alpha(source)
	if alpha == nil then
		return false, alpha_err
	end
	local output = self:output_cache(job, alpha)
	if not fs.cha(output) then
		ok, err = self:invert(source, output, alpha)
		if not ok or err then
			return ok, err
		end
	end

	return ya.image_precache(output, cache)
end

function M:cache(job)
	local base = ya.file_cache(job)
	return base and Url(base .. "-invert") or nil
end

function M:source_cache(job)
	local base = ya.file_cache(job)
	return base and Url(base .. "-invert-src") or nil
end

function M:output_cache(job, alpha)
	local base = ya.file_cache(job)
	if not base then
		return nil
	end
	return Url(base .. (alpha and "-invert.png" or "-invert.jpg"))
end

function M:rasterize(job, cache)
	if not cache then
		return false, Err("Failed to resolve intermediate image cache")
	elseif fs.cha(cache) then
		return true
	end

	local ext = self:ext(job.file.url)
	if ext == "svg" or ext == "svgz" then
		return self:rasterize_svg(job, cache)
	elseif self:is_magick_ext(ext) then
		return self:rasterize_magick(job, cache)
	end

	local ok, err = ya.image_precache(job.file.url, cache)
	if ok then
		return true
	end
	return self:rasterize_magick(job, cache)
end

function M:rasterize_magick(job, cache)
	local cmd = M.with_limit():arg(tostring(job.file.path))
	if job.args.flatten then
		cmd:arg("-flatten")
	end
	cmd:arg { "-auto-orient", "-strip" }

	local size = string.format("%dx%d>", rt.preview.max_width, rt.preview.max_height)
	if rt.preview.image_filter == "nearest" then
		cmd:arg { "-sample", size }
	elseif rt.preview.image_filter == "catmull-rom" then
		cmd:arg { "-filter", "catrom", "-thumbnail", size }
	elseif rt.preview.image_filter == "lanczos3" then
		cmd:arg { "-filter", "lanczos", "-thumbnail", size }
	elseif rt.preview.image_filter == "gaussian" then
		cmd:arg { "-filter", "gaussian", "-thumbnail", size }
	else
		cmd:arg { "-filter", "triangle", "-thumbnail", size }
	end

	cmd:arg { "-quality", rt.preview.image_quality }
	if job.args.bg then
		cmd:arg { "-background", job.args.bg, "-alpha", "remove" }
	end

	local status, err = cmd:arg(string.format("JPG:%s", cache)):status()
	if not status then
		return true, Err("Failed to start `magick`, error: %s", err)
	elseif not status.success then
		return false, Err("`magick` exited with error code: %s", status.code)
	else
		return true
	end
end

function M:rasterize_svg(job, cache)
	local cmd = Command("resvg"):arg {
		"-w",
		rt.preview.max_width,
		"-h",
		rt.preview.max_height,
		"--image-rendering",
		"optimizeSpeed",
	}
	if job.args.bg then
		cmd = cmd:arg { "--background", job.args.bg }
	end
	if rt.tasks.image_alloc > 0 then
		cmd = cmd:memory(rt.tasks.image_alloc)
	end

	local child, err = cmd:arg({ tostring(job.file.path), tostring(cache) }):spawn()
	if not child then
		return true, Err("Failed to start `resvg`, error: %s", err)
	end

	local status
	if rt.tasks.image_alloc == 0 then
		status, err = child:wait()
	end

	while not status and not err do
		ya.sleep(0.2)

		status, err = child:try_wait()
		if status or err then
			break
		end

		local id, mem = child:id(), nil
		if id then
			mem = ya.proc_info(id).mem_resident
		end
		if mem and mem > rt.tasks.image_alloc then
			child:start_kill()
			err = Err("memory limit exceeded, pid: %s, memory: %s", id, mem)
		end
	end

	if not status then
		return true, Err("Error while running `resvg`: %s", err)
	elseif not status.success then
		return false, Err("`resvg` exited with error code: %s", status.code)
	else
		return true
	end
end

function M:invert(source, output, alpha)
	local cmd = M.with_limit():arg(tostring(source))
	if alpha then
		cmd:arg { "-channel", "RGB", "-negate", "+channel" }
	else
		cmd:arg("-negate")
		cmd:arg { "-quality", rt.preview.image_quality }
	end

	local kind = alpha and "PNG:" or "JPG:"
	local status, err = cmd:arg(kind .. tostring(output)):status()
	if not status then
		return false, Err("Failed to start `magick`, error: %s", err)
	elseif not status.success then
		return false, Err("Failed to invert image, exit code: %s", status.code)
	else
		return true
	end
end

function M:ext(url)
	local ext = url.ext
	return ext and tostring(ext):lower() or ""
end

function M:has_alpha(source)
	local output, err = M.with_limit():arg({ tostring(source), "-format", "%[opaque]", "info:" }):output()

	if not output then
		return nil, Err("Failed to inspect image opacity: %s", err)
	elseif not output.status.success then
		return nil, Err("Failed to inspect image opacity, exit code: %s, stderr: %s", output.status.code, output.stderr)
	end

	return output.stdout:lower():find("false", 1, true) ~= nil
end

function M:is_magick_ext(ext)
	return ext == "avif" or ext == "heic" or ext == "heif" or ext == "heics" or ext == "heifs" or ext == "jxl"
end

function M.with_limit()
	local cmd = Command("magick"):arg { "-limit", "thread", 1 }
	if rt.tasks.image_alloc > 0 then
		cmd:arg { "-limit", "memory", rt.tasks.image_alloc, "-limit", "disk", "1MiB" }
	end
	if rt.tasks.image_bound[1] > 0 then
		cmd:arg { "-limit", "width", rt.tasks.image_bound[1] }
	end
	if rt.tasks.image_bound[2] > 0 then
		cmd:arg { "-limit", "height", rt.tasks.image_bound[2] }
	end
	return cmd
end

return M
