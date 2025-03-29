use rquickjs::Context;
use std::fs;
use std::path::Path;
use url::Url;

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
    ctx: Context,
}

impl ProxyResolver for PACResolver {
    fn resolve(&self, url: &str) -> String {
        let parsed = Url::parse(url).unwrap();
        let host = parsed.host_str().unwrap_or("");

        self.ctx
            .with(|ctx| {
                let globals = ctx.globals();
                let find_proxy_for_url: rquickjs::Function = globals
                    .get("FindProxyForURL")
                    .expect("Missing FindProxyForURL in PAC file");

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

fn get_resolver(settings: &ProxySettings) -> Resolver {
    if let Some(pac_url) = &settings.auto_config_url {
        println!("PAC URL: {}", pac_url);
        let pac_script = load_pac_script(&pac_url).expect("Could not load PAC script");

        let rt = rquickjs::Runtime::new().unwrap();
        let context = Context::full(&rt).unwrap();

        context.with(|ctx| {
            // Parse PAC source code
            let globals = ctx.globals();
            bind_pac_methods(&globals);
            ctx.eval::<(), _>(pac_script).expect("PAC script error");
        });
        Box::new(PACResolver { ctx: context })
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

fn run_lua(lua_path: &Path, proxy: &str, resolver: Resolver) {
    if lua_path.exists() {
        let lua = Lua::new();
        let lua_globals = lua.globals();

        lua_globals.set("proxy", proxy).unwrap();

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

fn main() {
    // Example URL to test proxy resolution
    let test_url = "https://github.com";

    let settings = get_proxy_settings().expect("Failed to load Windows proxy settings");

    let resolver = get_resolver(&settings);

    let proxy_result = resolver.resolve(test_url);

    let lua_path = Path::new("proxy.lua");
    run_lua(lua_path, &proxy_result, resolver);

    println!("\nProxy for {}: {}", test_url, proxy_result);
}
