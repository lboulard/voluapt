## Voluapt: use lua scripts to resolve proxy settings

Voluapt is able to find or use runtime HTTP proxy and provide resolution of URL
to find which HTTP proxy to use from lua scripts. On Windows, Internet Settings
are read from Windows registry. On Linux, you can provide a PAC file URL, or
local PAC file, to find HTTP proxy to use.

### Usage

Simplest usage is to obtain HTTP proxy from a PAC file:

```console
# voluapt --pac ~/.local/share/proxy.pac https://example.com
PROXY proxy.corp:8080
# voluapt --pac ~/.local/share/proxy.pac https://cvs.internal.corp
DIRECT
```

On Windows, where Internet Settings for user are read:

```dosbatch
▶ voluapt https://example.com
PROXY proxy.corp:8080
▶ voluapt https://cvs.internal.corp
DIRECT
```

Note that a lua script is not required in this case.

Lua scripts become important when you need to declare access to a list of URL.
You just change Voluapt command line invocation with new parameters. No need to
change lua scripts. When another PAC file is published, you just have to run
Voluapt to update proxy settings file created from lua scripts.

### Use static HTTP proxy

You can declare a static HTTP proxy configuration is a PAC file is not
applicable:

```console
# voluapt --proxy proxy.corp:8080 https://example.com
PROXY proxy.corp:8080
# voluapt --proxy proxy.corp:8080 --bypass *.corp https://cvs.internal.corp
DIRECT
```

#### Bypass list

You need to declare a bypass list to be able to detect internal sites. Use
a sequence of `--bypass *.corp --bypass *.oldcorp` to have Voluapt to return
direct access to all sites ending in `.corp` or `.oldcorp`.


#### Bypass list and PAC file

You can also force a bypass list when resolver is a PAC file. Hence, you will
always obtain a DIRECT connection for a bypassed domain if you need. In this
case, PAC resolver is not called.

### Lua scripts for Voluapt

Script in lua receive a `context` metatable in global with following fields:

- `context.find_proxy_for_url(url)`: function to resolve proxy for a URL
- `context.url`: non nil when URL is given to program argument
- `context.proxy`: result of proxy resolution on `context.url`
- `context.bypass_list`:
      Array from `--bypass` arguments from command line (or from Windows
      Internet Setting when proxy is activated)
- `context.defines`: key/value as defined from command line -D option
- `context.dns_resolve(hostname)`: function to resolve DNS address to IPv4

Note that `bypass_list` on Windows is a concatenation of bypass list in
Internet Settings and bypass given on command line with `--bypass`.

Using `-Dkey=value` command line argument, you can give external argument to
lua scripts. Scripts can access to string value in `context.defines` metatable.

There are two main strategies when using lua scripts:

1. Give a URL on command line to generate system shell scripts
2. Define proxy per URL from a list for git configuration file creation

For (1), combined with bypass list, from a single source, you have an easy way
to obtain dosbatch (`.bat`, `.cmd`), PowerShell (`.ps1`) or POSIX shell scripts
tailored from internal network HTTP environment.

For (2), allow git access only to a limited list of hosts. Limiting number of
host that can access external sites reduces hostile usage of local git
installation as malware transport.

Review examples in [lua/README.md](./lua/README.md) to see simple but efficient
usage.
