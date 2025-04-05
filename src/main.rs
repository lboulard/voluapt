use rquickjs::Context;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::exit;
use url::Url;

use clap::{ArgAction, Parser};
use mlua::Lua;

mod proxyjs;
use proxyjs::*;

mod fnmatch;
use fnmatch::fnmatch;

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

fn get_resolver(settings: &ProxySettings, verbose: bool, trace: bool) -> Resolver {
    let proxy_override = settings.proxy_override.clone().unwrap_or_default();
    let by_pass = proxy_override
        .split(&[';', ','])
        .collect::<Vec<&str>>()
        .iter()
        .map(|s| s.to_string())
        .collect();

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
            by_pass,
        })
    } else if settings.proxy_enable {
        let static_proxy = StaticResolver {
            proxy_server: settings.proxy_server.clone().unwrap_or_default(),
            by_pass,
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

fn run_lua(
    lua_path: &Path,
    url_proxy: Option<(String, String)>,
    resolver: Resolver,
    args: &Vec<(String, String)>,
) {
    if lua_path.exists() {
        let lua = Lua::new();
        let lua_globals = lua.globals();

        match url_proxy {
            Some((url, proxy)) => {
                lua_globals.set("url", url).unwrap();
                lua_globals.set("proxy", proxy).unwrap();
            }
            None => {}
        };

        if !args.is_empty() {
            let lua_args = lua.create_table().unwrap();
            for arg in args {
                lua_args.set(arg.0.as_str(), arg.1.as_str()).unwrap();
            }
            lua_globals.set("args", &lua_args).unwrap();
        }
        lua_globals
            .set("by_pass_list", resolver.no_proxy())
            .unwrap();

        // Register find_proxy_for_url in Lua
        let find_proxy_fn = lua
            .create_function(move |_, url: String| Ok(resolver.resolve(&url)))
            .unwrap();
        lua_globals
            .set("find_proxy_for_url", find_proxy_fn)
            .unwrap();

        // Register dns_resolve in Lua
        let dns_resolve_fn = lua
            .create_function(|_, host: String| Ok(resolve_dns(&host).unwrap_or_default()))
            .unwrap();
        lua_globals.set("dns_resolve", dns_resolve_fn).unwrap();

        lua.load(&fs::read_to_string(lua_path).expect("Failed to read Lua script"))
            .exec()
            .expect("Lua script execution failed");
    } else {
        eprintln!("Lua script not found: {}", lua_path.display());
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about)]
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
    #[arg(long)]
    static_proxy: Option<String>,

    /// Ignore proxy configuration for those site. Accept '*' pattern. Repeat for multiple bypass.
    #[arg(action = ArgAction::Append)]
    bypass_proxy: Option<String>,

    /// trace JavaScript for PAC
    #[arg(short = 't')]
    trace: bool,

    /// verbose message on error output
    #[arg(short = 'v')]
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
    proxy_override: Option<String>,
    verbose: bool,
    trace: bool,
) -> Result<Resolver, Box<dyn Error>> {
    let settings = match (pac, static_proxy, proxy_override) {
        (Some(pac), None, proxy_override) => Ok::<_, Box<dyn Error>>(ProxySettings {
            auto_config_url: Some(format!("file://{}", pac)),
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
        (None, None, _) => get_proxy_settings().map_err(Into::into),
    }?;

    Ok(get_resolver(&settings, verbose, trace))
}

fn main() {
    let args = Args::parse();

    // validate program arguments
    let (url, lua) = match (&args.pac, &args.static_proxy, &args.bypass_proxy) {
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
        args.bypass_proxy,
        args.verbose,
        args.trace,
    ) {
        Ok(resolver) => resolver,
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
