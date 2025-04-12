local context = context or {}

if context.url then
	proxy = context.proxy
else
	-- when no URL argument was given to voluapt invocation
	proxy = context.find_proxy_for_url("https://example.com")
end

bypass = coroutine.wrap(function()
	coroutine.yield("10.0.0.0/8")
	coroutine.yield("172.16.0.0/12")
	coroutine.yield("192.168.0.0/16")
	local no_proxy
	for _, no_proxy in pairs(context.bypass_list) do
		if no_proxy then
			no_proxy = no_proxy:gsub("^*.", "")
			if no_proxy ~= "" then
				coroutine.yield(no_proxy)
			end
		end
	end
end)

defines = context.defines or {}
outfile = defines and defines.outfile ~= "" and defines.outfile
if outfile then
	f = io.open(outfile, "w+")
else
	f = io.output()
end

if proxy ~= "DIRECT" then
	local proxy_host = proxy:gsub("PROXY ", "")
	f:write("proxy = " .. proxy .. "\n")

	local noproxy = {}
	for item in bypass do
		noproxy[1 + #noproxy] = item
	end
	if #noproxy > 0 then
		f:write("noproxy = " .. table.concat(noproxy, ",") .. "\n")
	end
else
	f:write("# no proxy required")
end
f:close()
