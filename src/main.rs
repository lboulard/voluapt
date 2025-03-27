use url::Url;

mod proxyjs;
use proxyjs::*;

mod fnmatch;
use fnmatch::fnmatch;

fn main() {
    // Example URL to test proxy resolution
    let test_url = "http://github.com";
    let parsed = Url::parse(test_url).unwrap();
    let host = parsed.host_str().unwrap_or("");

    let mut proxy_result = String::from("DIRECT");

    let settings = get_proxy_settings().expect("Failed to load Windows proxy settings");

    if let Some(pac_url) = &settings.auto_config_url {
        println!("PAC URL: {}", pac_url);
        let pac_script = load_pac_script(&pac_url).expect("Could not load PAC script");
        let ctx = create_pac_context(&pac_script);

        proxy_result = find_proxy(&ctx, test_url, host).unwrap_or("DIRECT".to_string());
    } else if settings.proxy_enable {
        let bypass = settings.proxy_override.clone().unwrap_or_default();
        let bypass_hosts: Vec<&str> = bypass.split(';').collect();
        let is_bypassed = bypass_hosts.iter().any(|pattern| fnmatch(pattern, host));

        if is_bypassed {
            proxy_result = "DIRECT".to_string();
        } else if let Some(proxy) = &settings.proxy_server {
            proxy_result = format!("PROXY {}", proxy);
        }
    }
    println!("\nProxy for {}: {}", test_url, proxy_result);
}
