//! Plain-English explanations for Roblox disconnects and crashes.

use serde::{Deserialize, Serialize};

/// How Aeolus may react to a disconnect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejoinPolicy {
    /// Connection problems: safe to rejoin, even automatically.
    Auto,
    /// Offer a Rejoin button, but never rejoin on its own (kicks, idle).
    Ask,
    /// Rejoining won't help (banned, account playing elsewhere, outdated client).
    Never,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Explanation {
    pub title: &'static str,
    pub detail: &'static str,
    pub policy: RejoinPolicy,
}

/// Explain a disconnect. `None` means Roblox didn't log a reason.
pub fn disconnect(code: Option<u32>) -> Explanation {
    use RejoinPolicy::*;
    let (title, detail, policy) = match code {
        Some(260) => ("Problem receiving data", "Roblox stopped getting data from the server. Usually a short internet or server hiccup.", Auto),
        Some(264) => ("Account joined from somewhere else", "The same account started a game on another device or window.", Never),
        Some(266) => ("Connection timed out", "The server stopped responding. Usually your internet or the server had a hiccup.", Auto),
        Some(267) => ("Kicked by the game", "The game removed you from the server. Its creator decides why (for example an admin or an anti-cheat).", Ask),
        Some(268) => ("Kicked for unexpected client behavior", "Roblox or the game removed you because something looked wrong. Mods only change sounds, cursors, fonts and skies, but if this keeps happening, try once with mods and Fast Flags off.", Ask),
        Some(273) => ("Account joined from somewhere else", "The same account joined a game from another device or window.", Never),
        Some(277) => ("Lost connection to the server", "Your connection to the game server dropped. Usually your internet or the server had a hiccup.", Auto),
        Some(278) => ("Disconnected for being idle", "Roblox disconnects you after about 20 minutes without any input.", Ask),
        Some(279) => ("Couldn't connect to the server", "Roblox couldn't reach the game server. A firewall, VPN or unstable internet can cause this.", Auto),
        Some(280) => ("Roblox is out of date", "This version of Roblox is too old for the server. Update Roblox from the Installations page.", Never),
        Some(282) => ("Disconnected from the game", "Roblox lost the game session. Rejoining usually works.", Auto),
        Some(524) => ("Not allowed to join", "You don't have permission to join this server (for example a private server you're not part of).", Never),
        Some(529) => ("Roblox is having trouble", "Roblox's servers returned an error. Check status.roblox.com if it keeps happening.", Auto),
        Some(600) => ("Banned from this game", "This game has banned your account, so rejoining won't work.", Never),
        Some(_) | None => ("Disconnected from the server", "Roblox lost the connection to the game. Rejoining usually works.", Auto),
    };
    Explanation { title, detail, policy }
}

/// What went wrong when Roblox closed unexpectedly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CrashReport {
    pub id: String,
    pub at: chrono::DateTime<chrono::Local>,
    /// e.g. "0xC0000005"
    pub exit_code: String,
    pub title: String,
    pub explanation: String,
    pub suggestions: Vec<String>,
    pub game: Option<String>,
}

/// Facts about the session that help explain a crash.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SessionFacts {
    /// Roblox wrote its normal shutdown lines to the log.
    pub clean_exit: bool,
    pub low_memory: bool,
    pub used_fast_flags: bool,
    pub used_mods: bool,
    pub game: Option<String>,
}

pub fn format_code(code: i32) -> String {
    format!("0x{:08X}", code as u32)
}

/// Decide whether an exit was a crash and explain it. `None` = normal exit.
pub fn crash(exit_code: Option<i32>, facts: &SessionFacts) -> Option<CrashReport> {
    let code = exit_code?;
    let u = code as u32;
    let exception = u >= 0xC000_0000 || (0x887A_0000..0x887B_0000).contains(&u);
    // Roblox exits with 0 when closed normally. A nonzero code with its normal
    // shutdown lines in the log is not treated as a crash.
    if code == 0 || (!exception && facts.clean_exit) {
        return None;
    }
    let (title, explanation, mut suggestions): (&str, &str, Vec<&str>) = match u {
        0xC000_0005 => (
            "Roblox crashed (memory access error)",
            "Roblox tried to use memory it wasn't allowed to. This is most often caused by graphics drivers or by overlay apps hooking into the game.",
            vec!["Update your graphics driver (NVIDIA, AMD or Intel).", "Close overlays that hook into games (Medal, MSI Afterburner/RivaTuner, Discord or GeForce overlay) and try again."],
        ),
        0xC000_0409 => (
            "Roblox stopped itself (security check)",
            "Roblox detected something unexpected in its own process and closed. Other programs that inject into games are the usual cause.",
            vec!["Close programs that inject into games (overlays, FPS unlockers, macro tools) and try again."],
        ),
        0xC000_0017 | 0xC000_012D | 0xE000_0008 => (
            "Roblox ran out of memory",
            "Your PC didn't have enough free memory for Roblox.",
            vec!["Close other programs, especially browsers with many tabs, before playing.", "Lower the graphics quality in Roblox's settings."],
        ),
        0xC000_0374 => (
            "Roblox crashed (memory corruption)",
            "Roblox's memory got corrupted. Drivers and programs that hook into games can cause this.",
            vec!["Update your graphics driver.", "Close overlays and programs that hook into games."],
        ),
        0x887A_0005 | 0x887A_0006 | 0x887A_0007 => (
            "Your graphics card stopped responding",
            "Windows reset the graphics driver while Roblox was using it (the graphics device was removed or hung).",
            vec!["Update your graphics driver.", "If your graphics card is overclocked, go back to stock settings.", "Lower the graphics quality in Roblox's settings."],
        ),
        _ if exception => (
            "Roblox crashed",
            "Roblox closed because of an error inside the game client.",
            vec!["Update your graphics driver.", "Close overlays and programs that hook into games."],
        ),
        _ => (
            "Roblox closed unexpectedly",
            "Roblox quit with an error code instead of closing normally.",
            vec![],
        ),
    };
    if facts.low_memory && !title.contains("memory") {
        suggestions.insert(0, "Your PC ran low on memory during this session. Close other programs before playing.");
    }
    if facts.used_fast_flags {
        suggestions.push("Try launching once with Fast Flags turned off (in your launch profile) to see if one of them causes it.");
    }
    if facts.used_mods {
        suggestions.push("Try once with mods turned off (launch profile → Use mods).");
    }
    suggestions.push("Run Verify on the Installations page to repair damaged Roblox files.");
    suggestions.push("If it keeps happening, reinstall Roblox from the Installations page.");
    Some(CrashReport {
        id: uuid::Uuid::new_v4().to_string(),
        at: chrono::Local::now(),
        exit_code: format_code(code),
        title: title.into(),
        explanation: explanation.into(),
        suggestions: suggestions.into_iter().map(String::from).collect(),
        game: facts.game.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disconnect_policies() {
        assert_eq!(disconnect(Some(277)).policy, RejoinPolicy::Auto);
        assert_eq!(disconnect(None).policy, RejoinPolicy::Auto);
        assert_eq!(disconnect(Some(267)).policy, RejoinPolicy::Ask);
        assert_eq!(disconnect(Some(278)).policy, RejoinPolicy::Ask);
        assert_eq!(disconnect(Some(600)).policy, RejoinPolicy::Never);
        assert_eq!(disconnect(Some(280)).title, "Roblox is out of date");
    }

    #[test]
    fn crash_classification() {
        let f = SessionFacts::default();
        assert!(crash(Some(0), &f).is_none());
        assert!(crash(None, &f).is_none());
        let clean = SessionFacts { clean_exit: true, ..Default::default() };
        assert!(crash(Some(1), &clean).is_none());
        assert_eq!(crash(Some(1), &f).unwrap().title, "Roblox closed unexpectedly");
        let av = crash(Some(0xC000_0005u32 as i32), &clean).unwrap();
        assert_eq!(av.exit_code, "0xC0000005");
        assert!(av.title.contains("memory access"));
        let facts = SessionFacts { low_memory: true, used_fast_flags: true, used_mods: true, game: Some("RIVALS".into()), ..Default::default() };
        let r = crash(Some(0x887A_0005u32 as i32), &facts).unwrap();
        assert!(r.title.contains("graphics"));
        assert!(r.suggestions[0].contains("low on memory"));
        assert!(r.suggestions.iter().any(|s| s.contains("Fast Flags")));
        assert!(r.suggestions.iter().any(|s| s.contains("mods")));
        assert_eq!(r.game.as_deref(), Some("RIVALS"));
    }
}
