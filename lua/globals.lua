-- display globals and system-proxy specific variables

local k, v

print("-- lua globals at top level")
for k, v in pairs(_ENV) do
	io.write(string.format("%-15s = %s\n", k, v))
end

print("-- resolved URL for proxy")
if url or proxy then
	io.write(string.format("%-15s = %s\n", "url", url))
	io.write(string.format("%-15s = %s\n", "proxy", proxy))
end

print("-- variable defined at command line using -D option")
if args then
	for k, v in pairs(args) do
		io.write(string.format("args.%-10s = %s\n", k, v))
	end
end
