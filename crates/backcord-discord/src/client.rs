//! How BACKCORD presents itself to Discord. The gateway and HTTP API both carry the
//! same description, matching a current desktop browser on the web client.

use std::{
    sync::{
        Mutex, OnceLock, PoisonError,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};

/// A local fake Discord for tests and measurements, such as
/// `http://127.0.0.1:18400`; see testing/fake_discord.ts. While it is set,
/// BACKCORD talks only to it, with [`TEST_TOKEN`] instead of the saved login.
pub fn test_server() -> Option<&'static str> {
    static SERVER: OnceLock<Option<String>> = OnceLock::new();
    SERVER
        .get_or_init(|| {
            let server = std::env::var("BACKCORD_DISCORD_TEST_SERVER").ok()?;
            let server = server.trim().trim_end_matches('/');
            (!server.is_empty()).then(|| server.to_owned())
        })
        .as_deref()
}

/// The token BACKCORD uses with the test server.
pub const TEST_TOKEN: &str = "backcord-test-token";

/// The Firefox release BACKCORD claims to be: one behind the newest, as most
/// people's browsers are. Firefox 155 came on 1 September 2026, and one has
/// come every two weeks since; revisit this if Mozilla changes the cadence
/// again.
fn firefox_version() -> i64 {
    const FIRST: i64 = 155;
    let first = chrono::NaiveDate::from_ymd_opt(2026, 9, 1).expect("a real date");
    let days = (chrono::Utc::now().date_naive() - first).num_days();
    FIRST + days.max(0) / 14 - 1
}

/// The `User-Agent` of a current Firefox on Linux.
pub fn user_agent() -> &'static str {
    static AGENT: OnceLock<String> = OnceLock::new();
    AGENT.get_or_init(|| {
        let version = firefox_version();
        if cfg!(windows) {
            format!(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:{version}.0) Gecko/20100101 Firefox/{version}.0"
            )
        } else {
            format!(
                "Mozilla/5.0 (X11; Linux x86_64; rv:{version}.0) Gecko/20100101 Firefox/{version}.0"
            )
        }
    })
}

/// The web client's language, sent in `X-Discord-Locale`: BACKCORD is in
/// English, as [`properties`]' `system_locale` says.
pub const LOCALE: &str = "en-US";

/// The `Accept-Language` the web client sends from an English Firefox's
/// `navigator.languages`, `["en-US", "en"]`.
pub const ACCEPT_LANGUAGE: &str = "en-US,en;q=0.9";

/// This machine's time zone, such as `Europe/Prague`, as the web client
/// sends it in `X-Discord-Timezone`; none when the system doesn't say.
pub fn timezone() -> Option<&'static str> {
    static ZONE: OnceLock<Option<String>> = OnceLock::new();
    ZONE.get_or_init(|| iana_time_zone::get_timezone().ok())
        .as_deref()
}

/// The web client's build number when BACKCORD last checked, for when its page
/// can't be read.
const CLIENT_BUILD_NUMBER: u64 = 637268;
/// The build number read from the web client's page, this run or the last
/// time it was read, or 0.
static BUILD_NUMBER: AtomicU64 = AtomicU64::new(0);
/// [`BUILD_NUMBER`] was read from the page this run.
static LEARNED: AtomicBool = AtomicBool::new(false);

/// The build number BACKCORD sends: the web client's current one once it's
/// known.
fn build_number() -> u64 {
    match BUILD_NUMBER.load(Ordering::Relaxed) {
        0 => CLIENT_BUILD_NUMBER,
        number => number,
    }
}

/// Whether the current build number was read this run.
pub(crate) fn knows_build_number() -> bool {
    LEARNED.load(Ordering::Relaxed)
}

/// Whether a build number read from the page is known: this run, or kept
/// from the last time it was read.
pub(crate) fn has_build_number() -> bool {
    BUILD_NUMBER.load(Ordering::Relaxed) != 0
}

/// Takes the build number kept from the last time the page was read,
/// unless one was read this run.
pub(crate) fn recall_build_number(number: u64) {
    let _ = BUILD_NUMBER.compare_exchange(0, number, Ordering::Relaxed, Ordering::Relaxed);
}

/// Takes the build number from the start of the web client's page, which
/// sets it in `GLOBAL_ENV`. Returns whether the page had one.
pub(crate) fn learn_build_number(page: &[u8]) -> bool {
    const KEY: &[u8] = br#""BUILD_NUMBER":""#;
    let Some(at) = page.windows(KEY.len()).position(|window| window == KEY) else {
        return false;
    };
    let digits = &page[at + KEY.len()..];
    let end = digits
        .iter()
        .position(|byte| !byte.is_ascii_digit())
        .unwrap_or(digits.len());
    let Some(number) = std::str::from_utf8(&digits[..end])
        .ok()
        .and_then(|digits| digits.parse::<u64>().ok())
        .filter(|&number| number > 0)
    else {
        return false;
    };
    BUILD_NUMBER.store(number, Ordering::Relaxed);
    LEARNED.store(true, Ordering::Relaxed);
    true
}

/// The build number known now: the one read from the page, or 0.
pub(crate) fn known_build_number() -> u64 {
    BUILD_NUMBER.load(Ordering::Relaxed)
}

/// A random UUID, as the web client makes them, with `cleared` bits unset.
fn uuid(cleared: u128) -> String {
    let mut bits = rand::random::<u128>();
    // Version 4, variant 1.
    bits = (bits & !(0xf << 76)) | (0x4 << 76);
    bits = (bits & !(0x3 << 62)) | (0x2 << 62);
    bits &= !cleared;
    let hex = format!("{bits:032x}");
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

/// Identifies this run of BACKCORD, as a launch of the client.
fn launch_id() -> &'static str {
    static ID: OnceLock<String> = OnceLock::new();
    ID.get_or_init(|| uuid(0))
}

/// The launch signature: a random UUID with the bits that report client
/// modifications unset.
fn launch_signature() -> &'static str {
    const MODIFICATIONS: u128 = 0x0080_1010_0810_0800_2081_0040_0100_0800;
    static SIGNATURE: OnceLock<String> = OnceLock::new();
    SIGNATURE.get_or_init(|| uuid(MODIFICATIONS))
}

/// The analytics heartbeat session, which the web client starts again every
/// 30 minutes.
fn heartbeat_session_id() -> String {
    const AGE: Duration = Duration::from_secs(30 * 60);
    static SESSION: Mutex<Option<(Instant, String)>> = Mutex::new(None);
    let mut session = SESSION.lock().unwrap_or_else(PoisonError::into_inner);
    if session
        .as_ref()
        .is_none_or(|(started, _)| started.elapsed() >= AGE)
    {
        *session = Some((Instant::now(), uuid(0)));
    }
    session
        .as_ref()
        .map(|(_, id)| id.clone())
        .unwrap_or_default()
}

/// The `properties` object sent when identifying on the gateway.
pub fn properties() -> Value {
    json!({
        "os": if cfg!(windows) { "Windows" } else { "Linux" },
        "browser": "Firefox",
        "device": "",
        "system_locale": "en-US",
        "has_client_mods": false,
        "browser_user_agent": user_agent(),
        "browser_version": format!("{}.0", firefox_version()),
        "os_version": if cfg!(windows) { "10" } else { "" },
        "referrer": "",
        "referring_domain": "",
        "referrer_current": "",
        "referring_domain_current": "",
        "release_channel": "stable",
        "client_build_number": build_number(),
        "client_event_source": null,
        "client_launch_id": launch_id(),
        "launch_signature": launch_signature(),
        "client_heartbeat_session_id": heartbeat_session_id(),
    })
}

/// The platforms Emulate Platforms can say BACKCORD is when it identifies
/// on the gateway, in its list's order; Discord shows the others where this
/// account is online from what it says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Emulated {
    Windows,
    Linux,
    MacOs,
    Android,
    Ios,
    Xbox,
    PlayStation,
    Vr,
    Web,
}

impl Emulated {
    pub const ALL: [Self; 9] = [
        Self::Windows,
        Self::Linux,
        Self::MacOs,
        Self::Android,
        Self::Ios,
        Self::Xbox,
        Self::PlayStation,
        Self::Vr,
        Self::Web,
    ];
}

/// How much of a platform's description Emulate Platforms sends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Depth {
    /// Its system, client and device, and locale.
    Low,
    /// And its client's version and release channel.
    Medium,
    /// And its user agent and build numbers.
    High,
}

/// The platform emulated, one past its index in [`Emulated::ALL`]; 0 for
/// none.
static EMULATED: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
static DEPTH: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// Emulate Platforms: the platform the next identify says, or none for
/// BACKCORD's own description, and how much of it.
pub fn set_emulation(platform: Option<Emulated>, depth: Depth) {
    let index = platform.map_or(0, |platform| {
        Emulated::ALL
            .iter()
            .position(|known| *known == platform)
            .unwrap_or(0)
            + 1
    });
    EMULATED.store(index as u8, Ordering::Relaxed);
    DEPTH.store(depth as u8, Ordering::Relaxed);
}

fn emulation() -> Option<(Emulated, Depth)> {
    let index = EMULATED.load(Ordering::Relaxed) as usize;
    let platform = *Emulated::ALL.get(index.checked_sub(1)?)?;
    let depth = match DEPTH.load(Ordering::Relaxed) {
        0 => Depth::Low,
        1 => Depth::Medium,
        _ => Depth::High,
    };
    Some((platform, depth))
}

/// What a platform's own client says of itself, as docs.discord.food shows
/// each: its system, client and device; its client's version, release
/// channel and system version; its user agent, architecture and native
/// build.
struct Described {
    os: &'static str,
    browser: &'static str,
    device: &'static str,
    client_version: Option<&'static str>,
    browser_version: &'static str,
    release_channel: &'static str,
    os_version: &'static str,
    user_agent: &'static str,
    arch: Option<&'static str>,
    client_build_number: Option<u64>,
    native_build_number: Option<u64>,
    design_id: Option<u64>,
}

fn described(platform: Emulated) -> Described {
    const DESKTOP: Described = Described {
        os: "Windows",
        browser: "Discord Client",
        device: "",
        client_version: Some("1.0.9219"),
        browser_version: "37.6.0",
        release_channel: "stable",
        os_version: "10.0.26100",
        user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) discord/1.0.9219 Chrome/138.0.7204.251 Electron/37.6.0 Safari/537.36",
        arch: Some("x64"),
        client_build_number: None,
        native_build_number: Some(72186),
        design_id: None,
    };
    const CONSOLE: Described = Described {
        os: "Xbox",
        browser: "Discord Embedded",
        device: "",
        client_version: None,
        browser_version: "0.0.8",
        release_channel: "unknown",
        os_version: "",
        user_agent: "Discord Embedded/0.0.8",
        arch: None,
        client_build_number: Some(4440),
        native_build_number: None,
        design_id: Some(0),
    };
    match platform {
        Emulated::Windows => DESKTOP,
        Emulated::Linux => Described {
            os: "Linux",
            client_version: Some("0.0.112"),
            os_version: "6.14.0",
            user_agent: "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) discord/0.0.112 Chrome/138.0.7204.251 Electron/37.6.0 Safari/537.36",
            native_build_number: None,
            ..DESKTOP
        },
        Emulated::MacOs => Described {
            os: "Mac OS X",
            client_version: Some("0.0.366"),
            os_version: "24.6.0",
            user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) discord/0.0.366 Chrome/138.0.7204.251 Electron/37.6.0 Safari/537.36",
            arch: Some("arm64"),
            native_build_number: None,
            ..DESKTOP
        },
        Emulated::Android => Described {
            os: "Android",
            browser: "Discord Android",
            device: "Pixel 8",
            client_version: Some("297.18 - rn"),
            browser_version: "",
            release_channel: "googleRelease",
            os_version: "35",
            user_agent: "",
            arch: None,
            client_build_number: Some(297018),
            native_build_number: None,
            design_id: Some(2),
        },
        Emulated::Ios => Described {
            os: "iOS",
            browser: "Discord iOS",
            device: "iPhone16,2",
            client_version: Some("297.0"),
            browser_version: "",
            release_channel: "stable",
            os_version: "18.6.2",
            user_agent: "",
            arch: None,
            client_build_number: Some(85871),
            native_build_number: None,
            design_id: Some(2),
        },
        Emulated::Xbox => CONSOLE,
        Emulated::PlayStation => Described {
            os: "Playstation",
            ..CONSOLE
        },
        Emulated::Vr => Described {
            os: "Android",
            browser: "Discord VR",
            device: "Quest 3",
            client_version: Some("297.18 - rn"),
            browser_version: "",
            release_channel: "googleRelease",
            os_version: "34",
            user_agent: "",
            arch: Some("arm64"),
            client_build_number: Some(297018),
            native_build_number: None,
            design_id: Some(2),
        },
        Emulated::Web => Described {
            os: if cfg!(windows) { "Windows" } else { "Linux" },
            browser: "Chrome",
            device: "",
            client_version: None,
            browser_version: "140.0.0.0",
            release_channel: "stable",
            os_version: if cfg!(windows) { "10" } else { "" },
            user_agent: if cfg!(windows) {
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36"
            } else {
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36"
            },
            arch: None,
            client_build_number: None,
            native_build_number: None,
            design_id: None,
        },
    }
}

/// `properties` as Emulate Platforms has them say `platform`, to
/// `depth`: the rest stays BACKCORD's own.
fn emulate(properties: &mut Value, platform: Emulated, depth: Depth) {
    let shown = described(platform);
    let Some(properties) = properties.as_object_mut() else {
        return;
    };
    let mut put = |key: &str, value: Value| {
        properties.insert(key.to_owned(), value);
    };
    put("os", shown.os.into());
    put("browser", shown.browser.into());
    put("device", shown.device.into());
    put("system_locale", LOCALE.into());
    if depth >= Depth::Medium {
        put("release_channel", shown.release_channel.into());
        put("os_version", shown.os_version.into());
        put("browser_version", shown.browser_version.into());
        if let Some(version) = shown.client_version {
            put("client_version", version.into());
        }
        if let Some(design) = shown.design_id {
            put("design_id", design.into());
        }
    }
    if depth >= Depth::High {
        put("browser_user_agent", shown.user_agent.into());
        if let Some(arch) = shown.arch {
            put("os_arch", arch.into());
            put("app_arch", arch.into());
        }
        if let Some(build) = shown.client_build_number {
            put("client_build_number", build.into());
        }
        if let Some(native) = shown.native_build_number {
            put("native_build_number", native.into());
        }
    }
}

/// The `properties` the gateway's identify sends: [`properties`], as
/// Emulate Platforms has them say another platform. HTTP requests keep
/// BACKCORD's own, which match the user agent they're sent with.
pub fn identify_properties() -> Value {
    let mut sent = properties();
    if let Some((platform, depth)) = emulation() {
        emulate(&mut sent, platform, depth);
    }
    sent
}

/// The same properties for HTTP requests' `X-Super-Properties` header.
pub fn super_properties() -> String {
    STANDARD.encode(properties().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emulating_a_platform_says_more_of_it_at_each_depth() {
        let mut low = properties();
        emulate(&mut low, Emulated::Android, Depth::Low);
        assert_eq!(low["os"], "Android");
        assert_eq!(low["browser"], "Discord Android");
        assert_eq!(low["device"], "Pixel 8");
        assert_eq!(low["release_channel"], "stable", "still BACKCORD's");
        let mut medium = properties();
        emulate(&mut medium, Emulated::Android, Depth::Medium);
        assert_eq!(medium["release_channel"], "googleRelease");
        assert_eq!(medium["client_version"], "297.18 - rn");
        assert_eq!(
            medium["browser_user_agent"],
            user_agent(),
            "still BACKCORD's"
        );
        let mut high = properties();
        emulate(&mut high, Emulated::Xbox, Depth::High);
        assert_eq!(high["os"], "Xbox");
        assert_eq!(high["browser"], "Discord Embedded");
        assert_eq!(high["browser_user_agent"], "Discord Embedded/0.0.8");
        assert_eq!(high["client_build_number"], 4440);
        // None is BACKCORD's own.
        set_emulation(None, Depth::High);
        assert_eq!(identify_properties()["browser"], "Firefox");
    }

    #[test]
    fn reads_the_build_number_from_the_web_page() {
        // One kept from the last start is sent until the page is read.
        recall_build_number(600000);
        assert!(has_build_number() && !knows_build_number());
        assert_eq!(build_number(), 600000);
        assert!(!learn_build_number(
            b"<script>window.GLOBAL_ENV = {}</script>"
        ));
        assert!(!learn_build_number(br#"{"BUILD_NUMBER":""}"#));
        assert!(learn_build_number(
            br#"GLOBAL_ENV = {"NODE_ENV":"production","BUILD_NUMBER":"637268","PROJECT_ENV":"production"}"#
        ));
        assert_eq!(build_number(), 637268);
        assert!(knows_build_number());
        recall_build_number(600000);
        assert_eq!(build_number(), 637268, "the one read this run stays");
    }

    #[test]
    fn launch_signatures_report_no_modifications() {
        let signature = launch_signature().replace('-', "");
        let bits = u128::from_str_radix(&signature, 16).unwrap();
        assert_eq!(bits & 0x0080_1010_0810_0800_2081_0040_0100_0800, 0);
        assert_eq!(launch_id().len(), 36);
        assert_eq!(&launch_id()[14..15], "4");
    }

    #[test]
    fn claims_a_firefox_from_this_year() {
        assert!(firefox_version() >= 157);
        assert!(user_agent().contains(&format!("Firefox/{}.0", firefox_version())));
    }
}
