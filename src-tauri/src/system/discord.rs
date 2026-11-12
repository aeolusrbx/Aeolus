//! Discord Rich Presence.
//!
//! While a Roblox session started by Aeolus is running, Discord shows
//! "Playing Aeolus · Running Roblox with Aeolus" with the Aeolus logo. Discord's
//! IPC is blocking, so a dedicated thread owns the connection; the rest of the
//! app sends it commands. If Discord isn't running, nothing happens and the
//! worker retries quietly.

use crate::diagnostics::logging::Logger;
use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Discord application that owns the "Aeolus" name and the `aeolus` art asset.
pub const DISCORD_APP_ID: &str = "1570171109771313212";
pub const LARGE_IMAGE_KEY: &str = "aeolus";
pub const PROJECT_URL: &str = "https://github.com/aeolusrbx/Aeolus";

#[derive(Debug, Clone, PartialEq)]
pub struct PresenceInfo {
    pub details: String,
    pub state: Option<String>,
    /// Unix seconds; Discord shows "elapsed" from this.
    pub started_at: i64,
    /// Art asset key or https URL for the big image (defaults to the Aeolus logo).
    pub large_image: Option<String>,
    pub large_text: Option<String>,
    /// When set, the Aeolus logo is shown as the small badge.
    pub small_logo: bool,
    /// Up to two (label, url) buttons; "Get Aeolus" is added if there's room.
    pub buttons: Vec<(String, String)>,
}

enum Cmd {
    Set(Option<PresenceInfo>),
    Shutdown,
}

pub struct Presence {
    tx: Mutex<Option<Sender<Cmd>>>,
}

/// Build the presence text for a running session.
pub fn presence_for(profile: Option<&str>, show_profile: bool, started_at: i64) -> PresenceInfo {
    PresenceInfo {
        details: "Running Roblox with Aeolus".into(),
        state: profile.filter(|_| show_profile).map(|p| format!("Profile: {p}")),
        started_at,
        large_image: None,
        large_text: None,
        small_logo: false,
        buttons: vec![],
    }
}

/// Presence while in a specific game.
pub fn presence_for_game(game: &crate::gamewatch::GameVisit, show_location: bool) -> PresenceInfo {
    let name = game.name.clone().unwrap_or_else(|| "a Roblox game".into());
    let mut state = game.creator.as_ref().map(|c| format!("by {c}"));
    if show_location {
        if let Some(l) = &game.location {
            state = Some(match state {
                Some(s) => format!("{s} · {l}"),
                None => format!("Server in {l}"),
            });
        }
    }
    PresenceInfo {
        details: format!("Playing {name}"),
        state,
        started_at: game.started_at.timestamp(),
        large_image: game.icon_url.clone(),
        large_text: Some(name),
        small_logo: game.icon_url.is_some(),
        buttons: vec![("View game".into(), format!("https://www.roblox.com/games/{}", game.place_id))],
    }
}

impl Presence {
    pub fn start(logger: Arc<Logger>) -> Self {
        if DISCORD_APP_ID.is_empty() {
            logger.info("discord", "Rich Presence is unavailable: no Discord application id is configured in this build");
            return Self { tx: Mutex::new(None) };
        }
        let (tx, rx) = channel::<Cmd>();
        std::thread::Builder::new()
            .name("aeolus-discord".into())
            .spawn(move || {
                let mut client: Option<DiscordIpcClient> = None;
                let mut desired: Option<PresenceInfo> = None;
                let mut shown: Option<PresenceInfo> = None;
                loop {
                    match rx.recv_timeout(Duration::from_secs(20)) {
                        Ok(Cmd::Set(p)) => desired = p,
                        Ok(Cmd::Shutdown) | Err(RecvTimeoutError::Disconnected) => {
                            if let Some(mut c) = client.take() {
                                let _ = c.clear_activity();
                                let _ = c.close();
                            }
                            return;
                        }
                        Err(RecvTimeoutError::Timeout) => {}
                    }
                    if desired == shown {
                        continue;
                    }
                    match &desired {
                        None => {
                            if let Some(mut c) = client.take() {
                                let _ = c.clear_activity();
                                let _ = c.close();
                            }
                            shown = None;
                        }
                        Some(info) => {
                            if client.is_none() {
                                let mut c = DiscordIpcClient::new(DISCORD_APP_ID);
                                match c.connect() {
                                    Ok(()) => client = Some(c),
                                    Err(e) => {
                                        logger.debug("discord", format!("Discord is not reachable ({e}); will retry"));
                                        continue;
                                    }
                                }
                            }
                            let c = client.as_mut().expect("connected above");
                            let aeolus_text = concat!("Aeolus ", env!("CARGO_PKG_VERSION"));
                            let mut assets = activity::Assets::new()
                                .large_image(info.large_image.as_deref().unwrap_or(LARGE_IMAGE_KEY))
                                .large_text(info.large_text.as_deref().unwrap_or(aeolus_text));
                            if info.small_logo {
                                assets = assets.small_image(LARGE_IMAGE_KEY).small_text(aeolus_text);
                            }
                            let mut buttons: Vec<activity::Button> = info.buttons.iter().take(2).map(|(l, u)| activity::Button::new(l.as_str(), u.as_str())).collect();
                            if buttons.len() < 2 {
                                buttons.push(activity::Button::new("Get Aeolus", PROJECT_URL));
                            }
                            let mut act = activity::Activity::new()
                                .details(info.details.as_str())
                                .timestamps(activity::Timestamps::new().start(info.started_at))
                                .assets(assets)
                                .buttons(buttons);
                            if let Some(s) = &info.state {
                                act = act.state(s.as_str());
                            }
                            match c.set_activity(act) {
                                Ok(()) => {
                                    logger.info("discord", "Rich Presence shown");
                                    shown = desired.clone();
                                }
                                Err(e) => {
                                    // Discord was probably restarted; drop the connection and retry next tick.
                                    logger.debug("discord", format!("Could not update Rich Presence ({e}); reconnecting"));
                                    let _ = c.close();
                                    client = None;
                                }
                            }
                        }
                    }
                }
            })
            .ok();
        Self { tx: Mutex::new(Some(tx)) }
    }

    fn send(&self, cmd: Cmd) {
        if let Some(tx) = self.tx.lock().unwrap_or_else(|p| p.into_inner()).as_ref() {
            let _ = tx.send(cmd);
        }
    }

    pub fn set(&self, info: Option<PresenceInfo>) {
        self.send(Cmd::Set(info));
    }

    pub fn shutdown(&self) {
        self.send(Cmd::Shutdown);
        *self.tx.lock().unwrap_or_else(|p| p.into_inner()) = None;
    }

    pub fn available(&self) -> bool {
        !DISCORD_APP_ID.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presence_text() {
        let p = presence_for(Some("Competitive"), true, 100);
        assert_eq!(p.details, "Running Roblox with Aeolus");
        assert_eq!(p.state.as_deref(), Some("Profile: Competitive"));
        assert_eq!(presence_for(Some("Competitive"), false, 100).state, None);
        assert_eq!(presence_for(None, true, 100).state, None);
    }

    #[test]
    fn presence_worker_survives_without_discord() {
        let d = tempfile::tempdir().unwrap();
        let p = Presence::start(Arc::new(Logger::new(d.path().into())));
        p.set(Some(presence_for(None, true, 1)));
        p.shutdown();
    }
}
