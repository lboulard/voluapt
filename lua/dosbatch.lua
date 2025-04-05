local context = context or {}

local proxy, by_pass

if context.url then
	proxy = context.proxy
else
	-- when no url argument was given to system-proxy invocation
	proxy = context.find_proxy_for_url("https://example.com")
end

by_pass = context.by_pass_list

local proxy_host
if proxy == "DIRECT" then
	proxy_host = nil
else
	proxy_host = proxy:gsub("PROXY ", "")
end

local template = [[
@SET HTTP_PROXY=@@proxy@@
@SET HTTPS_PROXY=@@proxy@@
@SET FTP_PROXY=@@proxy@@
]]

local no_proxy_header = (
	proxy_host
		and [[
@SET NO_PROXY=localhost,127.0.0.1
@SET NO_PROXY=%NO_PROXY%,10.0.0.0/8,172.16.0.0/12,192.168.0.0/16
]]
	or [[
@SET NO_PROXY=
]]
)

local no_proxy_template = [[
@SET NO_PROXY=%NO_PROXY%,@@no_proxy@@
]]

local f = io.open("use-proxy.bat", "w+")

local s = template:gsub("@@proxy@@", proxy_host or "")
f:write(s)
f:write(no_proxy_header)

for _, no_proxy in pairs(by_pass) do
	if no_proxy then
		no_proxy = no_proxy:gsub("^*", "")
		if no_proxy ~= "" then
			s = no_proxy_template:gsub("@@no_proxy@@", no_proxy)
			f:write(s)
		end
	end
end

f:write(proxy_host and [[
@ECHO.Using %HTTP_PROXY% for HTTP
@ECHO.Using %HTTPS_PROXY% for HTTPS
]] or [[
@ECHO.No proxy for HTTP
@ECHO.No proxy for HTTPS
]])

f:close()
