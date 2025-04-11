## Lua script for Voluapt usage

### Script [global.lua](globals.lua)

Script will display on standard output lua globals, and content of `context`
global variable created by Voluapt.


### Script [print_proxy.lua](print_proxy.lua)

Example of calling `context.find_proxy_for_url()` function to find HTTP proxy
for a URL. Result is formatted and printed on standard output.


### Script [git-http-proxy.lua](git-http-proxy.lua)

From a predefined list of URL git match and a test URL, a git configuration
file will be generated with a series of definition like this one:

```gitconfig
[http "https://*.example.com"]
    proxy = http://proxy.corp:8080
```

Call this script with `-Doutfile=/path/to/file.gitconfig`, then
define an include entry in user global git configuration in `~/.gitconfig` or
`$XDG_CONFIG_HOME/git/config`:

```gitconfig
[include]
    path = /path/to/file.gitconfig
```


### Scripts [dosbatch.lua](dosbatch.lua) and [powershell.lua](powershell.lua)

Both scripts will generate content of environment variables: `HTTP_PROXY`,
`HTTPS_PROXY`, `FTP_PROXY` and `NO_PROXY`.

Use `-Doutfile=proxy-settings.bat` for dos batch, and,
`-Doutfile=proxy-settings.ps1` for PowerShell. Then, you can use them this way:

  - `call proxy-settings.bat` to define environment variables for console
  - `. /path/to/proxy-settings.ps1` to merge new environment variables

### Scripts miscellaneous

- [curlrc.lua](curlrc.lua):  sample for a `.curlrc` file
- [npmrc.lua](npmrc.lua): sample for a `.npmrc` file
- [wgetrc.lua](wgetrc.lua): sample for a `.wgetrc` file

Those tools do not support nested configuration file. Use those samples to
generate your own configuration.
