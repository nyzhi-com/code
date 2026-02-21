use anyhow::Result;
use serde::{Deserialize, Serialize};

const SERVICE_NAME: &str = "nyzhi";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
    pub provider: String,
}

pub fn store_token(provider: &str, token: &StoredToken) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE_NAME, provider)?;
    let json = serde_json::to_string(token)?;
    entry.set_password(&json)?;
    Ok(())
}

pub fn load_token(provider: &str) -> Result<Option<StoredToken>> {
    let entry = keyring::Entry::new(SERVICE_NAME, provider)?;
    match entry.get_password() {
        Ok(json) => {
            let token: StoredToken = serde_json::from_str(&json)?;
            Ok(Some(token))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn delete_token(provider: &str) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE_NAME, provider)?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}
