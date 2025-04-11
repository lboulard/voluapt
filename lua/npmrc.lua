local context = context or {}

if context.url then
	proxy = context.proxy
else
	-- when no URL argument was given to voluapt invocation
	proxy = context.find_proxy_for_url("https://example.com")
end

defines = context.defines or {}
outfile = defines and defines.outfile ~= "" and defines.outfile
if outfile then
	f = io.open(outfile, "w+")
else
	f = io.output()
end

if proxy ~= "DIRECT" then
	local proxy_host = proxy:gsub("PROXY ", "")
	f:write("proxy=" .. proxy .. "\n")
	f:write("https-proxy=" .. proxy .. "\n")
else
	f:write("# no proxy required")
end
f:close()
