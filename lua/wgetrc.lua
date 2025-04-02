if not url then
    -- when no url argument was given to system-proxy invocation
	proxy = find_proxy_for_url("https://example.com")
end
local f = io.open(".wgetrc", "w+")
if proxy ~= "DIRECT" then
	local proxy_host = proxy:gsub("PROXY ", "")
    f:write("use_proxy = on\n")
    f:write("http_proxy = " .. proxy_result .. "\n")
else
    f:write("# no proxy required")
end
f:close()
