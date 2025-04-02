if not url then
	-- when no url argument was given to system-proxy invocation
	proxy = find_proxy_for_url("https://example.com")
end

local proxy_host
if proxy == "DIRECT" then
	proxy_host = ""
else
	proxy_host = proxy:gsub("PROXY ", "")
end

local template = [[
$env:HTTP_PROXY="@@proxy@@"
$env:HTTPS_PROXY="@@proxy@@"
$env:FTP_PROXY="$env:HTTP_PROXY"
$env:NO_PROXY="localhost,127.0.0.1"
$env:NO_PROXY="$env:NO_PROXY,10.0.0.0/8,172.16.0.0/12,192.168.0.0/16"
]]

local bypass_template = [[
$env:NO_PROXY="$env:NO_PROXY,@@bypass@@"
]]

local f = io.open("use-proxy.ps1", "w+")

local s = template:gsub("@@proxy@@", proxy_host)
f:write(s)
if args and args.bypass then
    s = bypass_template:gsub("@@bypass@@", args.bypass)
	f:write(s)
end
f:write([[
Write-Output "Using $env:HTTP_PROXY for HTTP"
Write-Output "Using $env:HTTPS_PROXY for HTTPS"
]])

f:close()
