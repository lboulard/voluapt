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

local proxy_url = context.proxy_to_url(proxy)

local template = [[
$env:HTTP_PROXY="@@proxy@@"
$env:HTTPS_PROXY="@@proxy@@"
$env:FTP_PROXY="@@proxy@@"
]]

local no_proxy_header = (
	proxy_url
		and [[
$env:NO_PROXY="localhost,127.0.0.1"
$env:NO_PROXY="$env:NO_PROXY,10.0.0.0/8,172.16.0.0/12,192.168.0.0/16"
]]
	or [[
$env:NO_PROXY=""
]]
)

local no_proxy_template = [[
$env:NO_PROXY="$env:NO_PROXY,@@no_proxy@@"
]]

defines = context.defines or {}
outfile = defines and defines.outfile ~= "" and defines.outfile
if outfile then
	f = io.open(outfile, "w+")
else
	f = io.output()
end

local s = template:gsub("@@proxy@@", proxy_url)
f:write(s)
f:write(no_proxy_header)

for no_proxy in bypass do
	s = no_proxy_template:gsub("@@no_proxy@@", no_proxy)
	f:write(s)
end

f:write(proxy_url and [[
Write-Output "Using $env:HTTP_PROXY for HTTP"
Write-Output "Using $env:HTTPS_PROXY for HTTPS"
]] or [[
Write-Output "No proxy for HTTP"
Write-Output "No proxy for HTTPS"
]])

f:close()
