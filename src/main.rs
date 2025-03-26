use std::io::ErrorKind;
use std::{fs, net::ToSocketAddrs};
use url::Url;

use rquickjs::function::Func;
use rquickjs::{Context, Runtime};

use ureq::Agent;
use winreg::RegKey;
use winreg::enums::*;

use std::io;
use std::net::UdpSocket;

fn get_my_ip_address() -> Result<String, io::Error> {
    // Trick: connect to a public IP to get the local IP (does not send packets)
    let socket = UdpSocket::bind("0.0.0.0:0")?;

    // Connect to a known remote address (no data is actually sent)
    socket.connect("8.8.8.8:80")?;

    // Step 3: Get the local socket address
    let local_addr = socket.local_addr()?;

    Ok(local_addr.ip().to_string())
}

fn is_in_net(ip: &str, pattern: &str, mask: &str) -> bool {
    use std::net::Ipv4Addr;

    let ip = ip.parse::<Ipv4Addr>().ok();
    let pattern = pattern.parse::<Ipv4Addr>().ok();
    let mask = mask.parse::<Ipv4Addr>().ok();

    match (ip, pattern, mask) {
        (Some(ip), Some(pat), Some(mask)) => {
            let ip_u32 = u32::from(ip);
            let pat_u32 = u32::from(pat);
            let mask_u32 = u32::from(mask);

            (ip_u32 & mask_u32) == (pat_u32 & mask_u32)
        }
        _ => false,
    }
}

fn dns_domain_is(host: &str, domain: &str) -> bool {
    host.ends_with(domain)
}

// DNS resolver using Windows API (supports IPv4 and IPv6)
fn resolve_dns(host: &str) -> Result<Option<String>, io::Error> {
    let addr_iter = (host, 0).to_socket_addrs()?;
    for addr in addr_iter {
        return Ok(Some(addr.ip().to_string()));
    }
    Ok(None)
}

fn is_plain_host_name(host: &str) -> bool {
    !host.contains('.')
}

fn fnmatch(pattern: &str, text: &str) -> bool {
    fn helper(pat: &[u8], txt: &[u8]) -> bool {
        if pat.is_empty() {
            return txt.is_empty();
        }

        match pat[0] {
            b'?' => {
                // ? matches any single character
                if txt.is_empty() {
                    false
                } else {
                    helper(&pat[1..], &txt[1..])
                }
            }
            b'*' => {
                // * matches zero or more characters
                helper(&pat[1..], txt) || (!txt.is_empty() && helper(pat, &txt[1..]))
            }
            _ => {
                // exact character match
                if txt.is_empty() || pat[0] != txt[0] {
                    false
                } else {
                    helper(&pat[1..], &txt[1..])
                }
            }
        }
    }

    helper(pattern.as_bytes(), text.as_bytes())
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
// Handle crappy server response like zscloud that break connection
fn load_pac_script(pac_url: &str) -> Option<String> {
    if pac_url.starts_with("http") {
        let agent = Agent::new();
        let resp = agent.get(pac_url).call();
        match resp {
            Err(error) => {
                println!("HTTP error: URL {}: {:?}", pac_url, error);
                Err(error).ok()?
            }
            Ok(response) => {
                // capture response to be able to respond for UnexpectedEof case
                let mut buf = String::new();
                match response.into_reader().read_to_string(&mut buf) {
                    Ok(_) => Some(buf),
                    Err(io_error) => {
                        if io_error.kind() == ErrorKind::UnexpectedEof {
                            // println!("Unexpected EOF: URL {}: {:?}", pac_url, io_error);
                            Some(buf)
                        } else {
                            println!("IO Error: URL {}: {:?}", pac_url, io_error);
                            Err(io_error).ok()?
                        }
                    }
                }
            }
        }
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
                Func::from(|host: String| match resolve_dns(&host) {
                    Ok(Some(response)) => {
                        println!("dnsResolve: {} ({})", host, response);
                        response
                    }
                    Ok(None) => {
                        println!("dnsResolve: {} (no response)", host);
                        "".to_string()
                    }
                    Err(e) => {
                        println!("dnsResolve: {} (error: {})", host, e);
                        "".to_string()
                    }
                }),
            )
            .unwrap();

        global
            .set(
                "dnsDomainIs",
                Func::from(|host: String, domain: String| {
                    let accepted = dns_domain_is(&host, &domain);
                    println!("dnDomainIs: {} {} ({})", host, domain, accepted,);
                    accepted
                }),
            )
            .unwrap();

        global
            .set(
                "shExpMatch",
                Func::from(|input: String, pattern: String| {
                    let accepted = fnmatch(&pattern, &input);
                    println!("shExpMath: {} | {} ({})", input, pattern, accepted);
                    accepted
                }),
            )
            .unwrap();

        global
            .set(
                "myIpAddress",
                Func::from(|| match get_my_ip_address() {
                    Ok(ip) => {
                        println!("myIpAddress: {}", ip);
                        ip
                    }
                    Err(e) => {
                        println!("myIpAddress: [failed] {}", e);
                        "127.0.0.1".to_string()
                    }
                }),
            )
            .unwrap();

        global
            .set(
                "isInNet",
                Func::from(|ip: String, pattern: String, mask: String| {
                    let accepted = is_in_net(&ip, &pattern, &mask);
                    println!("isInNet: {} | {}/{} ({})", ip, pattern, mask, accepted);
                    accepted
                }),
            )
            .unwrap();

        global
            .set(
                "isPlainHostName",
                Func::from(|host: String| {
                    let accepted = is_plain_host_name(&host);
                    println!("isPlainHostName: {} ({})", host, accepted);
                    accepted
                }),
            )
            .unwrap();

        ctx.eval::<(), _>(pac_script).unwrap();
    });

    ctx
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
    let test_url = "http://github.com";
    let parsed = Url::parse(test_url).unwrap();
    let host = parsed.host_str().unwrap_or("");

    let proxy = find_proxy(&ctx, test_url, host).unwrap_or("DIRECT".to_string());
    println!("\nProxy for {}: {}", test_url, proxy);
}
