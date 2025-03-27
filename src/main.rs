use url::Url;

mod proxyjs;
use proxyjs::*;

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
