//! Refresh tokens in the system keychain (Keychain, Credential Manager, Secret Service),
//! one entry per Google account. Settings only ever hold the email.

use super::GoogleError;

const SERVICE: &str = "ai.intelcad.ruz.google";

fn entry(email: &str) -> Result<keyring::Entry, GoogleError> {
    keyring::Entry::new(SERVICE, email).map_err(|e| GoogleError::Keychain(e.to_string()))
}

pub fn save(email: &str, refresh_token: &str) -> Result<(), GoogleError> {
    entry(email)?.set_password(refresh_token).map_err(|e| GoogleError::Keychain(e.to_string()))
}

pub fn load(email: &str) -> Result<String, GoogleError> {
    match entry(email)?.get_password() {
        Ok(token) => Ok(token),
        Err(keyring::Error::NoEntry) => Err(GoogleError::SignedOut),
        Err(e) => Err(GoogleError::Keychain(e.to_string())),
    }
}

pub fn delete(email: &str) {
    if let Ok(entry) = entry(email) {
        let _ = entry.delete_credential();
    }
}
