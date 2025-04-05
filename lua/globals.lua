-- display globals and system-proxy specific variables

local context = context or {}

print("-- lua globals at top level")
for k, v in pairs(_ENV) do
	io.write(string.format("%-15s = %s\n", k, v))
end

print("-- proxy context")
if context or context then
	for k, v in pairs(context) do
		io.write(string.format("context.%-15s = %s\n", k, v))
	end
end

print("-- variable defined at command line using -D option")
if context.defines then
	for k, v in pairs(context.defines) do
		io.write(string.format("context.defines.%-10s = %s\n", k, v))
	end
end
