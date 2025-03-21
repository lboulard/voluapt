use std::{fs, net::ToSocketAddrs};
use url::Url;

use rquickjs::function::Func;
use rquickjs::{Context, Runtime};

use ureq::Agent;
use winreg::RegKey;
use winreg::enums::*;

use regex::Regex;

// DNS resolver using Windows API (supports IPv4 and IPv6)
fn resolve_dns(host: &str) -> Option<String> {
    let addr_iter = (host, 0).to_socket_addrs().ok()?;
    for addr in addr_iter {
        return Some(addr.ip().to_string());
    }
    None
}

// Get the system PAC file URL or file path
fn get_pac_url() -> Option<String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let settings = hkcu
        .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings")
        .ok()?;

    let auto_config_url: Result<String, _> = settings.get_value("AutoConfigURL");
    if let Ok(url) = auto_config_url {
        Some(url)
    } else {
        None
    }
}

// Download or read PAC file
fn load_pac_script(pac_url: &str) -> Option<String> {
    if pac_url.starts_with("http") {
        let agent = Agent::new();
        let resp = agent.get(pac_url).call().ok()?;
        Some(resp.into_string().ok()?)
    } else if pac_url.starts_with("file://") {
        let path = pac_url.trim_start_matches("file://");
        fs::read_to_string(path).ok()
    } else {
        fs::read_to_string(pac_url).ok()
    }
}

fn create_pac_context(pac_script: &str) -> Context {
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();

    ctx.with(|ctx| {
        let global = ctx.globals();

        // Wrap closures with Func::from
        global
            .set(
                "dnsResolve",
                Func::from(|host: String| resolve_dns(&host).unwrap_or_default()),
            )
            .unwrap();

        global
            .set(
                "dnsDomainIs",
                Func::from(|host: String, domain: String| host.ends_with(&domain)),
            )
            .unwrap();

        global
            .set(
                "shExpMatch",
                Func::from(|input: String, pattern: String| {
                    let regex = glob_to_regex(&pattern);
                    regex.is_match(&input)
                }),
            )
            .unwrap();

        ctx.eval::<(), _>(pac_script).unwrap();
    });

    ctx
}

fn glob_to_regex(glob: &str) -> Regex {
    let mut pattern = String::from("^");
    for c in glob.chars() {
        match c {
            '*' => pattern.push_str(".*"),
            '?' => pattern.push('.'),
            '.' => pattern.push_str("\\."),
            _ => pattern.push(c),
        }
    }
    pattern.push('$');
    Regex::new(&pattern).unwrap()
}

fn find_proxy(ctx: &Context, url: &str, host: &str) -> Option<String> {
    ctx.with(|ctx| {
        let global = ctx.globals();
        let func: rquickjs::Function = global.get("FindProxyForURL").ok()?;
        func.call((url.to_string(), host.to_string())).ok()
    })
}

fn main() {
    let pac_url = get_pac_url().expect("PAC URL not found in system settings");
    println!("PAC URL: {}", pac_url);

    let pac_script = load_pac_script(&pac_url).expect("Could not load PAC script");

    let ctx = create_pac_context(&pac_script);

    // Example URL to test proxy resolution
    let test_url = "http://example.com";
    let parsed = Url::parse(test_url).unwrap();
    let host = parsed.host_str().unwrap_or("");

    let proxy = find_proxy(&ctx, test_url, host).unwrap_or("DIRECT".to_string());
    println!("Proxy for {}: {}", test_url, proxy);
}
