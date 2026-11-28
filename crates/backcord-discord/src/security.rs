//! This account's security, as Discord's "My Account" and "Devices" settings
//! show it: the devices signed in to it, from `GET /auth/sessions`, which
//! `POST /auth/sessions/logout` signs out; an authenticator app's secret for
//! turning two-factor authentication on, which the person scans or types
//! into their app; and the backup codes Discord gives with it. See
//! https://docs.discord.food/authentication#auth-session-object and
//! https://docs.discord.food/resources/user#enable-totp-mfa.

use serde_json::Value;

/// Discord's code for a password `POST /users/@me/mfa/totp/enable` took
/// without a secret: the setup goes on to the secret.
pub const PASSWORD_CHECKED: u64 = 60005;

/// A device signed in to this account: one of Discord's auth sessions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
    /// The session's hash, which signing it out names.
    pub id_hash: String,
    /// Its operating system, client or browser and rough location, as
    /// Discord gives them.
    pub os: Option<String>,
    pub platform: Option<String>,
    pub location: Option<String>,
    /// When it was last used, roughly, as Discord's ISO 8601 timestamp.
    pub last_used: Option<String>,
}

/// The devices in `GET /auth/sessions`'s answer, in its order.
pub fn devices(answer: &Value) -> Vec<Device> {
    let text = |value: &Value| {
        value
            .as_str()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_owned)
    };
    answer["user_sessions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|session| {
            let info = &session["client_info"];
            Some(Device {
                id_hash: text(&session["id_hash"])?,
                os: text(&info["os"]),
                platform: text(&info["platform"]),
                location: text(&info["location"]),
                last_used: text(&session["approx_last_used_time"]),
            })
        })
        .collect()
}

/// One of the account's backup codes, and whether it was used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackupCode {
    pub code: String,
    pub consumed: bool,
}

/// The backup codes in an answer's `backup_codes`.
pub fn backup_codes(answer: &Value) -> Vec<BackupCode> {
    answer["backup_codes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|code| {
            Some(BackupCode {
                code: code["code"].as_str()?.to_owned(),
                consumed: code["consumed"].as_bool().unwrap_or(false),
            })
        })
        .collect()
}

/// RFC 4648's base32 alphabet, which authenticator apps read secrets in.
const BASE32: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// A new secret for an authenticator app, as Discord takes it: 20 random
/// bytes, 32 characters of base32.
pub fn new_totp_secret() -> String {
    let bytes: [u8; 20] = rand::random();
    base32(&bytes)
}

fn base32(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(5) * 8);
    let (mut buffer, mut bits) = (0_u32, 0_u32);
    for byte in bytes {
        buffer = (buffer << 8) | u32::from(*byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(char::from(BASE32[((buffer >> bits) & 31) as usize]));
        }
    }
    if bits > 0 {
        out.push(char::from(BASE32[((buffer << (5 - bits)) & 31) as usize]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn devices_keep_what_discord_says_of_each() {
        let answer = json!({"user_sessions": [
            {"id_hash": "a=", "approx_last_used_time": "2024-03-30T21:30:58.100231+00:00",
             "client_info": {"os": "Windows", "platform": "Discord Client", "location": "Rome, Italy"}},
            {"id_hash": "b=", "client_info": {"os": null, "platform": " ", "location": null}},
            {"client_info": {}},
        ]});
        let found = devices(&answer);
        assert_eq!(found.len(), 2, "one without a hash can't be signed out");
        assert_eq!(found[0].os.as_deref(), Some("Windows"));
        assert_eq!(found[0].platform.as_deref(), Some("Discord Client"));
        assert_eq!(found[0].location.as_deref(), Some("Rome, Italy"));
        assert!(found[0].last_used.is_some());
        assert_eq!(found[1].platform, None, "blank is none");
    }

    #[test]
    fn a_secret_is_32_characters_of_base32() {
        let secret = new_totp_secret();
        assert_eq!(secret.len(), 32);
        assert!(secret.bytes().all(|byte| BASE32.contains(&byte)));
        assert_ne!(secret, new_totp_secret());
        // RFC 4648's test vector.
        assert_eq!(base32(b"foobar"), "MZXW6YTBOI");
    }

    #[test]
    fn backup_codes_say_which_were_used() {
        let answer = json!({"backup_codes": [
            {"user_id": "1", "code": "zqs8oqxk", "consumed": false},
            {"user_id": "1", "code": "abcd1234", "consumed": true},
        ]});
        assert_eq!(
            backup_codes(&answer),
            [
                BackupCode {
                    code: "zqs8oqxk".into(),
                    consumed: false
                },
                BackupCode {
                    code: "abcd1234".into(),
                    consumed: true
                },
            ]
        );
    }
}
