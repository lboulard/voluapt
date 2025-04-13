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

local proxy_url = context.proxy_to_url(proxy)

if proxy_url == "" then
	f:write("# no proxy required")
else
	f:write("proxy=" .. proxy_url .. "\n")
	f:write("https-proxy=" .. proxy_url .. "\n")
end
f:close()
