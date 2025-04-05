-- example to resolve proxy inside lua script

local url = "https://github.com/org/project"
local message = string.format('[%s]:%s', url, context.find_proxy_for_url(url))
print(message)
