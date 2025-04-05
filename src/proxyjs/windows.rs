use std::{error, fmt};

use crate::proxyjs::ProxySettings;

#[cfg(windows)]
use winreg::RegKey;

#[cfg(windows)]
use winreg::enums::*;

#[cfg(windows)]
use std::io;

#[cfg(windows)]
#[derive(Debug)]
pub struct ProxySettingsError {
    pub message: String,
    source: io::Error,
}

#[cfg(windows)]
impl fmt::Display for ProxySettingsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.message, self.source)
    }
}

#[cfg(windows)]
impl error::Error for ProxySettingsError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(&self.source)
    }
}

#[cfg(windows)]
pub fn get_proxy_settings() -> Result<ProxySettings, ProxySettingsError> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let settings = hkcu
        .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings")
        .map_err(|e| ProxySettingsError {
            message: "Windows Internet Settings".into(),
            source: e,
        })?;

    let auto_config_url = settings.get_value("AutoConfigURL").ok();
    let proxy_server = settings.get_value("ProxyServer").ok();
    let proxy_enable = settings.get_value::<u32, _>("ProxyEnable").unwrap_or(0) != 0;
    let proxy_override_string = settings.get_value::<String, _>("ProxyOverride").ok();

    let proxy_override = match proxy_override_string {
        Some(bypass) => bypass
            .split(';')
            .collect::<Vec<&str>>()
            .iter()
            .map(|s| s.to_string())
            .collect(),
        None => vec![],
    };

    Ok(ProxySettings {
        auto_config_url,
        proxy_enable,
        proxy_server,
        proxy_override,
    })
}
