use rquickjs::Context;
use url::Url;

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
        let context = rquickjs::Context::full(&rt).unwrap();

        context.with(|ctx| {
            // Parse PAC souce code
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

fn main() {
    // Example URL to test proxy resolution
    let test_url = "http://github.com";

    let settings = get_proxy_settings().expect("Failed to load Windows proxy settings");

    let resolver = get_resolver(&settings);

    let proxy_result = resolver.resolve(test_url);

    println!("\nProxy for {}: {}", test_url, proxy_result);
}
