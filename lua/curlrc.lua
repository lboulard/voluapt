if not url then
    -- when no url argument was given to system-proxy invocation
	proxy = find_proxy_for_url("https://example.com")
end
local f = io.open(".curlrc", "w+")
if proxy ~= "DIRECT" then
	local proxy_host = proxy:gsub("PROXY ", "")
	f:write("-x " .. proxy .. "\n")
else
    f:write("# no proxy required")
end
f:close()
