use std::io;
use std::{error, fmt};

use crate::proxyjs::ProxySettings;

#[cfg(unix)]
#[derive(Debug)]
pub struct ProxySettingsError {
    pub message: String,
    source: io::Error,
}

#[cfg(unix)]
impl fmt::Display for ProxySettingsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.message, self.source)
    }
}

#[cfg(unix)]
impl error::Error for ProxySettingsError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(&self.source)
    }
}

#[cfg(unix)]
pub fn get_proxy_settings() -> Result<ProxySettings, ProxySettingsError> {
    Ok(ProxySettings {
        auto_config_url: None,
        proxy_enable: false,
        proxy_server: None,
        proxy_override: None,
    })
}
