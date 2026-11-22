//! Security: a proxy every connection to Discord goes through, and encrypted
//! DNS (DNS over HTTPS) for looking its servers up. Both take effect when
//! BACKCORD starts.

use iced::{Element, widget::text};

use super::{Pref, pref};
use crate::settings::{Event, Page};
use crate::{
    app::Message,
    controls::{Kind, action, choices},
    form,
    layout::Prefs,
    pictures::Icon,
};

/// The proxy's field.
pub const PROXY_FIELD: &str = "security-proxy";
/// The field for a DNS server of one's own.
pub const DNS_FIELD: &str = "security-dns";

/// The encrypted DNS servers offered, after the system's own lookups, and
/// their JSON APIs; the last choice is a server of one's own.
pub const DNS: [(&str, &str); 3] = [
    ("Cloudflare", "https://1.1.1.1/dns-query"),
    ("Google", "https://8.8.8.8/resolve"),
    ("Quad9", "https://9.9.9.9:5053/dns-query"),
];
const CUSTOM: usize = DNS.len() + 1;

/// The proxy and the encrypted DNS server `prefs` ask for, as
/// `backcord_core::discord::set_network` takes them.
pub fn network(prefs: &Prefs) -> (Option<String>, Option<String>) {
    let proxy = prefs
        .proxy
        .as_deref()
        .map(str::trim)
        .filter(|proxy| {
            prefs.proxy_on.unwrap_or(false) && backcord_core::discord::valid_proxy(proxy)
        })
        .map(str::to_owned);
    let dns = match prefs.dns.unwrap_or(0) {
        0 => None,
        CUSTOM => prefs
            .dns_custom
            .as_deref()
            .map(str::trim)
            .filter(|server| server.starts_with("https://"))
            .map(str::to_owned),
        index => DNS.get(index - 1).map(|(_, server)| (*server).to_owned()),
    };
    (proxy, dns)
}

impl<'a> Page<'a> {
    pub(in crate::settings) fn security_page(self) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let prefs = &self.cx.layout.prefs;
        let tr = backcord_iced_kit::i18n::tr;
        let proxy_on = prefs.proxy_on.unwrap_or(false);
        let proxy = prefs.proxy.as_deref().unwrap_or_default();
        let mut parts = vec![
            form::title(theme, tr("Security")),
            self.flip(
                tr("Use a proxy"),
                tr("Messages, pictures and the connection to Discord go through it. A call's sound and video go straight to Discord's voice servers: proxies don't carry them."),
                proxy_on,
                Pref::ProxyOn,
            ),
        ];
        if proxy_on {
            let mut group = vec![
                form::heading(theme, tr("PROXY ADDRESS")),
                form::field(theme, PROXY_FIELD, "socks5://127.0.0.1:1080", proxy)
                    .on_input(|address| pref(Pref::Proxy(address)))
                    .into(),
            ];
            if !proxy.trim().is_empty() && !backcord_core::discord::valid_proxy(proxy) {
                group.push(
                    form::error(theme, tr("Write it as socks5://, http:// or https://, then the host and the port, such as socks5://127.0.0.1:1080.")).into(),
                );
            } else {
                group.push(form::note(
                    theme,
                    tr("SOCKS5, HTTP or HTTPS. A name and password go before the host: socks5://name:password@host:port."),
                ));
            }
            parts.push(form::group(group));
        }
        let dns = prefs.dns.unwrap_or(0).min(CUSTOM);
        let mut names = vec![tr("System")];
        names.extend(DNS.iter().map(|(name, _)| *name));
        names.push(tr("Custom"));
        let mut group = vec![
            form::heading(theme, tr("ENCRYPTED DNS")),
            choices(theme, &names, dns, |index| pref(Pref::Dns(index))),
            form::note(
                theme,
                tr(
                    "Discord's servers are looked up over HTTPS, so your network and your provider don't see or change which ones. System uses your computer's own lookups.",
                ),
            ),
        ];
        if dns == CUSTOM {
            let custom = prefs.dns_custom.as_deref().unwrap_or_default();
            group.push(
                form::field(theme, DNS_FIELD, "https://dns.example/dns-query", custom)
                    .on_input(|server| pref(Pref::DnsCustom(server)))
                    .into(),
            );
            if !custom.trim().is_empty() && !custom.trim().starts_with("https://") {
                group.push(
                    form::error(
                        theme,
                        tr("A DNS over HTTPS server's address starts with https://."),
                    )
                    .into(),
                );
            } else {
                group.push(form::note(
                    theme,
                    tr("Its JSON API, as Cloudflare and Google answer it."),
                ));
            }
        }
        parts.push(form::group(group));
        if network(prefs) != backcord_core::discord::network() {
            let ink = crate::theme::rgb(theme.text);
            let restart = action(
                theme,
                tr("Restart BACKCORD"),
                Kind::Plain,
                Some(Message::Settings(Event::Restart)),
            )
            .icon(self.cx.look.icon(Icon::Refresh, 18, ink));
            parts.push(form::beside(
                text(tr("The new settings take effect when BACKCORD restarts. Restarting leaves any call."))
                    .size(14)
                    .color(theme.text),
                restart,
            ));
        }
        form::page(parts)
    }
}
