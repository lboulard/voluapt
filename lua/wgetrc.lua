local context = context or {}

local http_proxy
if context.url then
	http_proxy = context.proxy
else
	-- when no URL argument was given to voluapt invocation
	http_proxy = context.find_proxy_for_url("https://example.com")
end

defines = context.defines or {}
outfile = defines and defines.outfile ~= "" and defines.outfile
if outfile then
	f = io.open(outfile, "w+")
else
	f = io.output()
end

if http_proxy ~= "DIRECT" then
	http_proxy = http_proxy:gsub("PROXY ", "")
	f:write("use_proxy = on\n")
	f:write("http_proxy = " .. http_proxy .. "\n")
else
	f:write("# no proxy required")
end
f:close()
