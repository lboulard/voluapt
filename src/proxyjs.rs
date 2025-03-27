use chrono::{Datelike, Local, Timelike};
use std::io::ErrorKind;
use std::{fs, net::ToSocketAddrs};

use rquickjs::function::Func;
use rquickjs::{Context, Runtime};

use ureq::Agent;
use winreg::RegKey;
use winreg::enums::*;

use std::io;
use std::net::UdpSocket;

use crate::fnmatch::fnmatch;

pub struct ProxySettings {
    pub auto_config_url: Option<String>,
    pub proxy_server: Option<String>,
    pub proxy_enable: bool,
    pub proxy_override: Option<String>,
}

pub fn get_proxy_settings() -> Option<ProxySettings> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let settings = hkcu
        .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings")
        .ok()?;

    let auto_config_url = settings.get_value("AutoConfigURL").ok();
    let proxy_server = settings.get_value("ProxyServer").ok();
    let proxy_enable = settings.get_value::<u32, _>("ProxyEnable").unwrap_or(0) != 0;
    let proxy_override = settings.get_value("ProxyOverride").ok();

    Some(ProxySettings {
        auto_config_url,
        proxy_server,
        proxy_enable,
        proxy_override,
    })
}

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

fn local_host_or_domain_is(host: &str, hostdom: &str) -> bool {
    host == hostdom || hostdom.starts_with(&format!("{}.", host))
}

fn weekday_range_js(args: Vec<String>) -> bool {
    let now = Local::now();
    let current_day = now.weekday().num_days_from_sunday();
    let days = ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"];
    let day_index = |d: &str| days.iter().position(|x| *x == d.to_uppercase());

    match args.len() {
        1 => day_index(&args[0]) == Some(current_day as usize),
        2 => {
            if let (Some(start), Some(end)) = (day_index(&args[0]), day_index(&args[1])) {
                if start <= end {
                    (start..=end).contains(&(current_day as usize))
                } else {
                    let mut range = (start..7).chain(0..=end);
                    range.any(|d| d == current_day as usize)
                }
            } else {
                false
            }
        }
        _ => false,
    }
}

fn time_range_js(args: Vec<u32>) -> bool {
    let hour = Local::now().hour();
    match args.len() {
        1 => hour == args[0],
        2 => {
            let (start, end) = (args[0], args[1]);
            if start <= end {
                hour >= start && hour <= end
            } else {
                hour >= start || hour <= end
            }
        }
        _ => false,
    }
}

fn date_range_js(args: Vec<u32>) -> bool {
    let now = Local::now();
    match args.len() {
        1 => now.day() == args[0],
        2 => now.month() == args[0] && now.day() == args[1],
        3 => now.month() == args[0] && now.day() == args[1] && now.year() as u32 == args[2],
        _ => false,
    }
}

// Download or read PAC file
// Handle crappy server response like zscloud that break connection
pub fn load_pac_script(pac_url: &str) -> Option<String> {
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

pub fn create_pac_context(pac_script: &str) -> Context {
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

        global
            .set(
                "localHostOrDomainIs",
                Func::from(|host: String, hostdom: String| {
                    local_host_or_domain_is(&host, &hostdom)
                }),
            )
            .unwrap();

        global
            .set(
                "weekdayRange",
                Func::from(|args: Vec<String>| weekday_range_js(args)),
            )
            .unwrap();

        global
            .set(
                "timeRange",
                Func::from(|args: Vec<u32>| time_range_js(args)),
            )
            .unwrap();

        global
            .set(
                "dateRange",
                Func::from(|args: Vec<u32>| date_range_js(args)),
            )
            .unwrap();

        global
            .set(
                "alert",
                Func::from(|msg: String| {
                    eprintln!("[PAC ALERT] {}", msg);
                }),
            )
            .unwrap();
        ctx.eval::<(), _>(pac_script).unwrap();
    });

    ctx
}

pub fn find_proxy(ctx: &Context, url: &str, host: &str) -> Option<String> {
    ctx.with(|ctx| {
        let global = ctx.globals();
        let func: rquickjs::Function = global.get("FindProxyForURL").ok()?;
        func.call((url.to_string(), host.to_string())).ok()
    })
}
