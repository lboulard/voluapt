use rquickjs::{Context, Function, Persistent};
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
}

struct PACResolver {
    context: Context,
    function: Persistent<Function<'static>>,
}

fn new_pac_resolver(context: Context) -> PACResolver {
    let function = context.with(|ctx| {
        let f: Function = ctx
            .globals()
            .get("FindProxyForURL")
            .expect("Missing FindProxyForURL in PAC file");
        Persistent::save(&ctx, f)
    });
    PACResolver { context, function }
}

impl ProxyResolver for PACResolver {
    fn resolve(&self, url: &str) -> String {
        let parsed = Url::parse(url).unwrap();
        let host = parsed.host_str().unwrap_or("");

        let function = self.function.clone();
        self.context
            .with(|ctx| {
                let find_proxy_for_url = function.restore(&ctx).unwrap();
                find_proxy_for_url.call((url.to_string(), host.to_string()))
            })
            .unwrap_or_default()
    }
}

struct DirectResolver;

impl ProxyResolver for DirectResolver {
    fn resolve(&self, _url: &str) -> String {
        "DIRECT".to_string()
    }
}

fn get_resolver(settings: &ProxySettings, trace: bool) -> Resolver {
    if let Some(pac_url) = &settings.auto_config_url {
        println!("PAC URL: {}", pac_url);
        let pac_script = load_pac_script(&pac_url).expect("Could not load PAC script");

        let rt = rquickjs::Runtime::new().unwrap();
        let context = Context::full(&rt).unwrap();

        context.with(|ctx| {
            // Parse PAC source code
            let globals = ctx.globals();
            bind_pac_methods(&globals, trace);
            ctx.eval::<(), _>(pac_script).expect("PAC script error");
        });
        Box::new(new_pac_resolver(context))
    } else if settings.proxy_enable {
        let bypass = settings.proxy_override.clone().unwrap_or_default();
        let bypass_hosts: Vec<&str> = bypass.split(';').collect();

        let static_proxy = StaticResolver {
            proxy_server: settings.proxy_server.clone().unwrap_or_default(),
            by_pass: bypass_hosts.iter().map(|s| s.to_string()).collect(),
        };
        Box::new(static_proxy)
    } else {
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

    /// trace JavaScript for PAC
    #[arg(short = 't')]
    trace: bool,
}

fn parse_key_val(s: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err("Define must be in KEY=VALUE format".into());
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

fn _resolver(trace: bool) -> Resolver {
    let settings = get_proxy_settings().expect("Failed to load Windows proxy settings");
    get_resolver(&settings, trace)
}

fn main() {
    let args = Args::parse();

    match (&args.url, &args.lua) {
        (Some(url), Some(lua_path)) => {
            let resolver = _resolver(args.trace);
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
            let resolver = _resolver(args.trace);
            let proxy_result = resolver.resolve(url);
            println!("{}", proxy_result);
        }
        (None, Some(lua_path)) => {
            let resolver = _resolver(args.trace);
            let lua_path = Path::new(lua_path);
            run_lua(lua_path, None, resolver, &args.defines);
        }
        (None, None) => {
            eprintln!("** ERROR : No URL specified, nor lua script to run.");
            exit(2);
        }
    }
}
