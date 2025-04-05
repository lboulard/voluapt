pub struct ProxySettings {
    pub auto_config_url: Option<String>,
    pub proxy_enable: bool,
    pub proxy_server: Option<String>,
    pub proxy_override: Vec<String>,
}
