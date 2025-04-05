-- create a git config file with proxy entry for each url
--
-- default to write on standard output
-- define savefile to save to a file with "-Dsavefile=/path/to/my/file"

local context = context or {}

local function git_http_proxy(writer, git_pattern, proxy)
	local is_proxy = string.match(proxy, "^PROXY%s+(.+)%s*$")
	if proxy == "DIRECT" then
		proxy = ""
	elseif is_proxy then
		proxy = " http://" .. is_proxy
	else
		proxy = " # ignoring unknown proxy string: " .. proxy
	end

	local http_proxy = string.format('[http "%s"]\n\tproxy =%s\n', git_pattern, proxy)
	writer:write(http_proxy)
end

local hosts_url = {
	-- simple url string
	"https://gitlab.com",
	-- table with {<url>, <git url pattern>}
	{ "https://gitlab.com", "https://*.gitlab.com" },

	"https://github.com",
	"https://gist.github.com",
	"https://bitbucket.org",
	"https://git.kernel.org",
	"https://9fans.net",
	{ "https://googlesource.com", "https://*.googlesource.com" },
	{ "http://golang.org", "http://golang.org" },
	"https://golang.org",
	"https://gopkg.in",
}

local writer, defines, outfile

defines = context.defines or {}
outfile = defines and defines.outfile ~= "" and defines.outfile
if outfile then
	writer = io.open(outfile, "w+")
else
	writer = io.output()
end

writer:setvbuf("line")
writer:write("# vim: set ft=gitconfig et ts=8 sts=8 sw=8:\n")
writer:write("\n")

for _, url_or_table in ipairs(hosts_url) do
	local url, git_pattern
	if type(url_or_table) == "table" then
		url = url_or_table[1]
		git_pattern = url_or_table[2]
	else
		url = tostring(url_or_table)
		git_pattern = url
	end
	local proxy = context.find_proxy_for_url(url)
	git_http_proxy(writer, git_pattern, proxy)
end
