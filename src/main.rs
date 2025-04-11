use rquickjs::Context;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::exit;
use url::Url;

use clap::{ArgAction, Parser};
use mlua::{Lua, Table};

mod proxyjs;
use proxyjs::*;

mod fnmatch;
use fnmatch::fnmatch;

fn is_alnum_or_hyphen(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

#[derive(Debug, PartialEq)]
pub enum ProxyParseError {
    UnexpectedToken,
    MissingAddress,
    InvalidAddress,
    InvalidPort,
}

struct ProxyParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> ProxyParser<'a> {
    fn new(input: &'a str) -> Self {
        ProxyParser { input, pos: 0 }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn advance(&mut self, expected: &str) -> bool {
        if self.input[self.pos..].starts_with(expected) {
            self.pos += expected.len();
            true
        } else {
            false
        }
    }

    fn advance_while<F>(&mut self, mut predicate: F) -> bool
    where
        F: FnMut(char) -> bool,
    {
        let mut advanced = false;
        while let Some(c) = self.peek() {
            if predicate(c) {
                self.pos += c.len_utf8();
                advanced = true;
            } else {
                break;
            }
        }
        advanced
    }

    fn advance_case_insensitive(&mut self, expected: &str) -> bool {
        let remaining = &self.input[self.pos..];

        // Take the same number of characters as the expected string
        let prefix: String = remaining.chars().take(expected.len()).collect();

        if prefix.len() != expected.len() {
            return false; // Not enough chars
        }

        if prefix.eq_ignore_ascii_case(expected) {
            self.pos += prefix.len();
            true
        } else {
            false
        }
    }

    fn skip_whitespace(&mut self) {
        self.advance_while(|c| c.is_whitespace());
    }

    fn only_padding_remains(&self) -> bool {
        self.input[self.pos..]
            .chars()
            .all(|c| c.is_whitespace() || c == ';')
    }

    fn finish_proxy(&self, start: usize) -> Result<String, ProxyParseError> {
        if self.only_padding_remains() {
            let addr = self.input[start..self.pos]
                .trim_end_matches(|c: char| c == ';' || c.is_whitespace());
            Ok(format!("PROXY {}", addr))
        } else {
            Err(ProxyParseError::UnexpectedToken)
        }
    }

    fn parse_server_addr(&mut self) -> Result<(), ProxyParseError> {
        let start = self.pos;

        // Try IP address
        if self.advance_digits_dot_quad() {
            return Ok(());
        }

        // Try hostname + optional port
        self.pos = start; // reset
        if self.advance_hostname() {
            if self.peek() == Some(':') {
                self.pos += 1;
                if !self.advance_while(|c| c.is_ascii_digit()) {
                    return Err(ProxyParseError::InvalidPort);
                }
            }
            return Ok(());
        }

        Err(ProxyParseError::MissingAddress)
    }

    fn parse(mut self) -> Result<String, ProxyParseError> {
        self.skip_whitespace();

        if self.advance_case_insensitive("DIRECT") {
            self.skip_whitespace();
            return if self.only_padding_remains() {
                Ok("DIRECT".to_string())
            } else {
                Err(ProxyParseError::UnexpectedToken)
            };
        }

        if self.advance_case_insensitive("PROXY") && self.advance(" ") {
            let start = self.pos;

            self.parse_server_addr()?; // returns early on error

            self.skip_whitespace();
            return self.finish_proxy(start);
        }

        Err(ProxyParseError::UnexpectedToken)
    }

    fn advance_digits_dot_quad(&mut self) -> bool {
        for i in 0..4 {
            if !self.advance_while(|c| c.is_ascii_digit()) {
                return false;
            }
            if i < 3 {
                if !self.advance(".") {
                    return false;
                }
            }
        }

        // Optional port
        if self.peek() == Some(':') {
            self.pos += 1;
            if !self.advance_while(|c| c.is_ascii_digit()) {
                return false;
            }
        }

        true
    }

    fn advance_hostname(&mut self) -> bool {
        if !self.advance_while(is_alnum_or_hyphen) {
            return false;
        }

        while self.peek() == Some('.') {
            self.pos += 1;
            if !self.advance_while(is_alnum_or_hyphen) {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_simple() {
        assert_eq!(ProxyParser::new("DIRECT").parse(), Ok("DIRECT".to_string()));
    }

    #[test]
    fn test_direct_with_whitespace() {
        assert_eq!(
            ProxyParser::new("   DIRECT   ").parse(),
            Ok("DIRECT".to_string())
        );
        assert_eq!(
            ProxyParser::new("  DIRECT;;").parse(),
            Ok("DIRECT".to_string())
        );
    }

    #[test]
    fn test_proxy_ip_no_port() {
        assert_eq!(
            ProxyParser::new("PROXY 192.168.0.1").parse(),
            Ok("PROXY 192.168.0.1".to_string())
        );
    }

    #[test]
    fn test_proxy_ip_with_port() {
        assert_eq!(
            ProxyParser::new("PROXY 192.168.0.1:8080").parse(),
            Ok("PROXY 192.168.0.1:8080".to_string())
        );
    }

    #[test]
    fn test_proxy_hostname_no_port() {
        assert_eq!(
            ProxyParser::new("PROXY proxy.example.com").parse(),
            Ok("PROXY proxy.example.com".to_string())
        );
    }

    #[test]
    fn test_proxy_hostname_with_port() {
        assert_eq!(
            ProxyParser::new("PROXY proxy.example.com:8080").parse(),
            Ok("PROXY proxy.example.com:8080".to_string())
        );
    }

    #[test]
    fn test_trailing_junk_ignored() {
        assert_eq!(
            ProxyParser::new(" PROXY proxy.example.com:8080 ;;  ").parse(),
            Ok("PROXY proxy.example.com:8080".to_string())
        );
    }

    #[test]
    fn test_invalid_prefix() {
        assert_eq!(
            ProxyParser::new("INVALID proxy.example.com").parse(),
            Err(ProxyParseError::UnexpectedToken)
        );
    }

    #[test]
    fn test_proxy_hostname_with_invalid_port() {
        assert_eq!(
            ProxyParser::new("PROXY proxy.example.com:abc").parse(),
            Err(ProxyParseError::InvalidPort)
        );
    }

    #[test]
    fn test_proxy_missing_hostname_or_ip() {
        assert_eq!(
            ProxyParser::new("PROXY ").parse(),
            Err(ProxyParseError::MissingAddress)
        );
    }

    #[test]
    fn test_is_alnum_or_hyphen_function() {
        assert!(is_alnum_or_hyphen('a'));
        assert!(is_alnum_or_hyphen('Z'));
        assert!(is_alnum_or_hyphen('9'));
        assert!(is_alnum_or_hyphen('-'));
        assert!(is_alnum_or_hyphen('_'));
        assert!(!is_alnum_or_hyphen('!'));
        assert!(!is_alnum_or_hyphen(' '));
    }
}

trait ProxyResolver {
    fn resolve(&self, url: &str) -> String;
    fn no_proxy(&self) -> Vec<String>;
}

type Resolver = Box<dyn ProxyResolver>;

struct StaticResolver {
    proxy_server: String,
    by_pass: Vec<String>,
}

impl ProxyResolver for StaticResolver {
    fn resolve(&self, url: &str) -> String {
        let proxy_result: String;

        let parsed = Url::parse(url).unwrap();
        let host = parsed.host_str().unwrap_or("");

        let is_bypassed = self.by_pass.iter().any(|pattern| fnmatch(pattern, host));

        if is_bypassed {
            proxy_result = "DIRECT".to_string();
        } else {
            proxy_result = format!("PROXY {}", self.proxy_server);
        }
        proxy_result
    }

    fn no_proxy(&self) -> Vec<String> {
        self.by_pass.clone()
    }
}

struct PACResolver {
    ctx: Context,
    by_pass: Vec<String>,
}

impl ProxyResolver for PACResolver {
    fn resolve(&self, url: &str) -> String {
        let parsed = Url::parse(url).unwrap();
        let host = parsed.host_str().unwrap_or("");

        let is_bypassed = self.by_pass.iter().any(|pattern| fnmatch(pattern, host));

        if is_bypassed {
            "DIRECT".to_string()
        } else {
            self.ctx
                .with(|ctx| {
                    let globals = ctx.globals();
                    let find_proxy_for_url: rquickjs::Function = globals
                        .get("FindProxyForURL")
                        .expect("Missing FindProxyForURL in PAC file");

                    let result = find_proxy_for_url.call((url.to_string(), host.to_string()));
                    ctx.run_gc();
                    result
                })
                .unwrap_or_default()
        }
    }

    fn no_proxy(&self) -> Vec<String> {
        self.by_pass.clone()
    }
}

struct DirectResolver;

impl ProxyResolver for DirectResolver {
    fn resolve(&self, _url: &str) -> String {
        "DIRECT".to_string()
    }

    fn no_proxy(&self) -> Vec<String> {
        vec![]
    }
}

struct SafeResolver {
    parent: Resolver,
}

impl ProxyResolver for SafeResolver {
    fn resolve(&self, _url: &str) -> String {
        let proxy = self.parent.resolve(_url);
        ProxyParser::new(&proxy)
            .parse()
            .unwrap_or_else(|err| panic!("[{}]: {:?}", proxy, err))
    }
    fn no_proxy(&self) -> Vec<String> {
        self.parent.no_proxy()
    }
}

fn make_safe_resolver(resolver: Resolver) -> Resolver {
    Box::new(SafeResolver { parent: resolver })
}

fn get_resolver(settings: &ProxySettings, verbose: bool, trace: bool) -> Resolver {
    if let Some(pac_url) = &settings.auto_config_url {
        if verbose {
            eprintln!("PAC_URL={}", pac_url);
        }
        let pac_script = load_pac_script(&pac_url).expect("Could not load PAC script");

        let rt = rquickjs::Runtime::new().unwrap();
        let context = Context::full(&rt).unwrap();

        context.with(|ctx| {
            // Parse PAC source code
            let globals = ctx.globals();
            bind_pac_methods(&globals, trace);
            ctx.eval::<(), _>(pac_script).expect("PAC script error");
        });
        Box::new(PACResolver {
            ctx: context,
            by_pass: settings.proxy_override.clone(),
        })
    } else if settings.proxy_enable {
        let static_proxy = StaticResolver {
            proxy_server: settings.proxy_server.clone().unwrap_or_default(),
            by_pass: settings.proxy_override.clone(),
        };

        if verbose {
            eprintln!("HTTP_PROXY={}", static_proxy.proxy_server);
            eprintln!("NO_PROXY={}", static_proxy.by_pass.join(","));
        }
        Box::new(static_proxy)
    } else {
        if verbose {
            eprintln!("HTTP_PROXY=");
            eprintln!("NO_PROXY=");
        }
        Box::new(DirectResolver)
    }
}

fn create_lua_context(
    lua: &Lua,
    url_proxy: Option<(String, String)>,
    resolver: Resolver,
    defines: &Vec<(String, String)>,
) -> Table {
    let context = lua.create_table().unwrap();

    match url_proxy {
        Some((url, proxy)) => {
            context.set("url", url).unwrap();
            context.set("proxy", proxy).unwrap();
        }
        None => {}
    };

    context.set("by_pass_list", resolver.no_proxy()).unwrap();

    let context_defines = lua.create_table().unwrap();
    for arg in defines {
        context_defines.set(arg.0.as_str(), arg.1.as_str()).unwrap();
    }
    context.set("defines", &context_defines).unwrap();

    // Register find_proxy_for_url in Lua
    let find_proxy_fn = lua
        .create_function(move |_, url: String| Ok(resolver.resolve(&url)))
        .unwrap();
    context.set("find_proxy_for_url", find_proxy_fn).unwrap();

    // Register dns_resolve in Lua
    let dns_resolve_fn = lua
        .create_function(|_, host: String| Ok(resolve_dns(&host).unwrap_or_default()))
        .unwrap();
    context.set("dns_resolve", dns_resolve_fn).unwrap();

    context
}

fn run_lua(
    lua_path: &Path,
    url_proxy: Option<(String, String)>,
    resolver: Resolver,
    args: &Vec<(String, String)>,
) {
    if lua_path.exists() {
        let lua = Lua::new();

        lua.globals()
            .set(
                "context",
                &create_lua_context(&lua, url_proxy, resolver, args),
            )
            .unwrap();

        lua.load(&fs::read_to_string(lua_path).expect("Failed to read Lua script"))
            .exec()
            .expect("Lua script execution failed");
    } else {
        eprintln!("Lua script not found: {}", lua_path.display());
    }
}

#[cfg(windows)]
macro_rules! platform_help {
    () => {
        "\
Use Internet Settings on Windows by default.\n\
Override default behaviour using --proxy for static HTTP PROXY,\n\
or use alternative PAC file with --pac.\n\
"
    };
}

#[cfg(unix)]
macro_rules! platform_help {
    () => {
        "\
One of --proxy for static HTTP PROXY, or --pac for PAC file is required.\n\
"
    };
}

macro_rules! after_help {
    () => {
        concat!(
            platform_help!(),
            r#"
PAC file can be a local file (with and without "file://" prefix),
or a HTTP/HTTPS url like "https://lan.corp/proxy.pac".

Option --bypass is used in PAC proxy resolver and static HTTP proxy.
In case a host match by-pass list, PAC script is not called.

When lua script is given, URL argument is optional.
If URL argument is present, proxy for URL is resolved.
Then proxy and URL are given in proxy context to lua script.

Script in lua receive a "context" in global with following fields:

  - context.find_proxy_for_url(url): function to resolve proxy for an URL
  - context.url: non nil when URL is given to program argument
  - context.proxy: result of proxy resolution on context.url
  - context.by_pass_list: table from by pass arguments from command line (or
                          Windows Internet Setting when proxy is activated)
  - context.defines: key/value as defined from command line -D option
  - context.dns_resolve(hostname): function to resolve DNS address to IPv4

Proxy response for find_proxy_for_url() or context.proxy, match those
patterns:

  - DIRECT
  - PROXY xxx.xxx.xxx.xxx:port (when IPv4 address)
  - PROXY example.proxy.corp:port (when using DNS to find proxy address)
"#
        )
    };
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(after_help = after_help!())]
struct Args {
    /// URL to resolve
    url: Option<String>,

    /// Lua script to run
    #[arg(long)]
    lua: Option<String>,

    /// Key=Value definitions for Lua
    #[arg(short = 'D', value_parser = parse_key_val, action = ArgAction::Append)]
    defines: Vec<(String, String)>,

    /// Provide PAC file manually, and ignore Internet Settings
    #[arg(long)]
    pac: Option<String>,

    /// Configure for a static HTTP proxy, mutually exclusive with --pac
    #[arg(long = "proxy")]
    static_proxy: Option<String>,

    /// Ignore proxy configuration for those site. Accept '*' pattern. Repeat for multiple bypass.
    #[arg(short='N', long="bypass", action = ArgAction::Append)]
    bypass: Vec<String>,

    /// trace JavaScript for PAC
    #[arg(short = 't', long = "trace")]
    trace: bool,

    /// verbose message on error output
    #[arg(short = 'v', long = "verbose")]
    verbose: bool,
}

fn parse_key_val(s: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err("Define must be in KEY=VALUE format".into());
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

fn find_resolver(
    pac: Option<String>,
    static_proxy: Option<String>,
    proxy_override: Vec<String>,
    verbose: bool,
    trace: bool,
) -> Result<Resolver, Box<dyn Error>> {
    let settings = match (pac, static_proxy, proxy_override) {
        (Some(pac), None, proxy_override) => Ok::<_, Box<dyn Error>>(ProxySettings {
            auto_config_url: Some(pac),
            proxy_enable: false,
            proxy_server: None,
            proxy_override,
        }),
        (None, Some(proxy_server), proxy_override) => Ok(ProxySettings {
            auto_config_url: None,
            proxy_enable: true,
            proxy_server: Some(proxy_server),
            proxy_override,
        }),
        (Some(_), Some(_), _) => Err("--pac and --static-proxy are mutually exclusive".into()),
        (None, None, proxy_override) => match get_proxy_settings().map_err(Into::into) {
            Ok(settings) => Ok(ProxySettings {
                auto_config_url: settings.auto_config_url,
                proxy_enable: settings.proxy_enable,
                proxy_server: settings.proxy_server,
                proxy_override: [settings.proxy_override, proxy_override].concat(),
            }),
            Err(e) => Err(e),
        },
    }?;

    Ok(get_resolver(&settings, verbose, trace))
}

fn main() {
    let args = Args::parse();

    // validate program arguments
    let (url, lua) = match (&args.pac, &args.static_proxy, &args.bypass) {
        (Some(_), Some(_), _) => Err("--pac and --static-proxy are mutually exclusive"),
        _ => Ok((None::<String>, None::<String>)),
    }
    .and(match (&args.url, &args.lua) {
        (None, None) => Err("no URL specified, nor lua script to run."),
        (url, lua) => Ok((url, lua)),
    })
    .unwrap_or_else(|error| {
        eprintln!(" ** ERROR : {}\n", error);
        exit(2)
    });

    let resolver = match find_resolver(
        args.pac,
        args.static_proxy,
        args.bypass,
        args.verbose,
        args.trace,
    ) {
        Ok(resolver) => make_safe_resolver(resolver),
        Err(message) => {
            eprintln!(" ** ERROR : {}\n", message);
            exit(1)
        }
    };

    match (&url, &lua) {
        (Some(url), Some(lua_path)) => {
            let proxy_result = resolver.resolve(url);
            let lua_path = Path::new(lua_path);
            run_lua(
                lua_path,
                Some((url.to_string(), proxy_result)),
                resolver,
                &args.defines,
            );
        }
        (Some(url), None) => {
            if !(args.url.is_none() || args.defines.is_empty()) {
                eprintln!("** WARNING : variable defined and no lua script to run");
            }
            let proxy_result = resolver.resolve(url);
            println!("{}", proxy_result);
        }
        (None, Some(lua_path)) => {
            let lua_path = Path::new(lua_path);
            run_lua(lua_path, None, resolver, &args.defines);
        }
        (None, None) => {
            unreachable!("no URL specified, nor lua script to run.");
        }
    }
}
