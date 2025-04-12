local context = context or {}

local proxy, bypass

if context.url then
	proxy = context.proxy
else
	-- when no URL argument was given to voluapt invocation
	proxy = context.find_proxy_for_url("https://example.com")
end

bypass = coroutine.wrap(function()
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

defines = context.defines or {}
outfile = defines and defines.outfile ~= "" and defines.outfile
if outfile then
	f = io.open(outfile, "w+")
else
	f = io.output()
end

local s = template:gsub("@@proxy@@", proxy_host or "")
f:write(s)
f:write(no_proxy_header)

for no_proxy in bypass do
	s = no_proxy_template:gsub("@@no_proxy@@", no_proxy)
	f:write(s)
end

f:write(proxy_host and [[
@ECHO.Using %HTTP_PROXY% for HTTP
@ECHO.Using %HTTPS_PROXY% for HTTPS
]] or [[
@ECHO.No proxy for HTTP
@ECHO.No proxy for HTTPS
]])

f:close()
