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

	local _, show_err = ya.image_show(cache, job.area)
	ya.preview_widget(job, show_err)
end

function M:seek(job) require("pdf"):seek(job) end

function M:preload(job, cache)
	cache = cache or self:cache(job)
	if not cache or fs.cha(cache) then
		return true
	end

	local source = self:source_cache(job)
	if not source then
		return false, Err("Failed to resolve intermediate PDF cache")
	end
	local source_image = self:source_image(job)
	if not source_image then
		return false, Err("Failed to resolve rasterized PDF cache")
	end

	if not fs.cha(source) then
		local ok, err, bound = require("pdf"):preload(job, source)
		if bound or not ok or err then
			return ok, err, bound
		end
	end

	local alpha, alpha_err = require("invert-image"):has_alpha(source_image)
	if alpha == nil then
		return false, alpha_err
	end

	local output = self:output_cache(job, alpha)
	if not output then
		return false, Err("Failed to resolve inverted PDF cache")
	end
	if not fs.cha(output) then
		local ok, err = require("invert-image"):invert(source_image, output, alpha)
		if not ok or err then
			return ok, err
		end
	end

	return ya.image_precache(output, cache)
end

function M:cache(job)
	local base = require("pdf"):cache(job)
	return base and Url(base .. "-invert") or nil
end

function M:source_cache(job)
	local base = require("pdf"):cache(job)
	return base and Url(base .. "-invert-src") or nil
end

function M:source_image(job)
	local source = self:source_cache(job)
	return source and Url(source .. ".jpg") or nil
end

function M:output_cache(job, alpha)
	local base = require("pdf"):cache(job)
	if not base then
		return nil
	end
	return Url(base .. (alpha and "-invert.png" or "-invert.jpg"))
end

return M
