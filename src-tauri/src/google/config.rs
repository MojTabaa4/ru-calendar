//! The Google OAuth client Ruz signs in with ("Desktop app" type, from Google Cloud).

use std::path::Path;

use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientConfig {
    pub client_id: String,
    /// Google requires it for desktop apps, but it can't be kept secret in one, so it
    /// grants nothing by itself; PKCE protects each sign-in.
    pub client_secret: String,
}

/// The JSON file Google Cloud Console offers for download.
#[derive(Deserialize)]
struct Download {
    installed: ClientConfig2,
}

#[derive(Deserialize)]
struct ClientConfig2 {
    client_id: String,
    client_secret: String,
}

pub const FILE_NAME: &str = "google-client.json";

pub fn parse_download(json: &str) -> Option<ClientConfig> {
    let d: Download = serde_json::from_str(json).ok()?;
    Some(ClientConfig { client_id: d.installed.client_id, client_secret: d.installed.client_secret })
}

fn from_pair(id: Option<String>, secret: Option<String>) -> Option<ClientConfig> {
    match (id, secret) {
        (Some(client_id), Some(client_secret)) if !client_id.is_empty() && !client_secret.is_empty() => {
            Some(ClientConfig { client_id, client_secret })
        }
        _ => None,
    }
}

/// First match wins: `RUZ_GOOGLE_CLIENT_ID` / `RUZ_GOOGLE_CLIENT_SECRET` in the
/// environment, then `google-client.json` in the app's config folder, then the same
/// variables at build time.
pub fn load(config_dir: &Path) -> Option<ClientConfig> {
    let env = |key: &str| std::env::var(key).ok();
    from_pair(env("RUZ_GOOGLE_CLIENT_ID"), env("RUZ_GOOGLE_CLIENT_SECRET"))
        .or_else(|| {
            std::fs::read_to_string(config_dir.join(FILE_NAME)).ok().as_deref().and_then(parse_download)
        })
        .or_else(|| {
            from_pair(
                option_env!("RUZ_GOOGLE_CLIENT_ID").map(str::to_owned),
                option_env!("RUZ_GOOGLE_CLIENT_SECRET").map(str::to_owned),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_console_download() {
        let json = r#"{"installed":{"client_id":"123-abc.apps.googleusercontent.com","project_id":"ruz",
            "auth_uri":"https://accounts.google.com/o/oauth2/auth","token_uri":"https://oauth2.googleapis.com/token",
            "client_secret":"GOCSPX-xyz","redirect_uris":["http://localhost"]}}"#;
        let client = parse_download(json).unwrap();
        assert_eq!(client.client_id, "123-abc.apps.googleusercontent.com");
        assert_eq!(client.client_secret, "GOCSPX-xyz");
        assert!(parse_download(r#"{"web":{}}"#).is_none(), "only Desktop app clients work");
    }
}
