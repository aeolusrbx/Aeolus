//! Game detection from Roblox's own log files.
//!
//! While a Roblox session started by Aeolus runs, a background thread follows
//! the newest `%LOCALAPPDATA%\Roblox\logs\*_Player_*_last.log` and turns these
//! lines into join/leave events:
//!
//! ```text
//! [FLog::Output] ! Joining game '<job id>' place <place id> at <internal ip>
//! [FLog::GameJoinLoadTime] Report game_join_loadtime: placeid:<id>, ... universeid:<id>
//! [FLog::Network] UDMUX Address = <public ip>, Port = <port> | RCC Server Address = ...
//! [FLog::Network] serverId: <ip>|<port>
//! [FLog::SingleSurfaceApp] leaveUGCGameInternal
//! ```
//!
//! Only these lines are read; nothing else from the log is stored.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Joining { job_id: String, place_id: u64 },
    Universe { place_id: u64, universe_id: u64 },
    PublicAddress { ip: String },
    Joined { ip: String },
    PrivateServer,
    ReservedServer,
    Left,
}

static JOINING: Lazy<Regex> = Lazy::new(|| Regex::new(r"! Joining game '([0-9a-fA-F\-]{36})' place (\d+) at ").unwrap());
static UNIVERSE: Lazy<Regex> = Lazy::new(|| Regex::new(r"game_join_loadtime: placeid:(\d+),.*?universeid:(\d+)").unwrap());
static UDMUX: Lazy<Regex> = Lazy::new(|| Regex::new(r"UDMUX Address = ([0-9]{1,3}(?:\.[0-9]{1,3}){3}), Port = \d+").unwrap());
static JOINED: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[FLog::Network\] serverId: ([0-9]{1,3}(?:\.[0-9]{1,3}){3})\|\d+").unwrap());

pub fn parse_line(line: &str) -> Option<Event> {
    if let Some(c) = JOINING.captures(line) {
        return Some(Event::Joining { job_id: c[1].to_ascii_lowercase(), place_id: c[2].parse().ok()? });
    }
    if let Some(c) = UNIVERSE.captures(line) {
        return Some(Event::Universe { place_id: c[1].parse().ok()?, universe_id: c[2].parse().ok()? });
    }
    if let Some(c) = UDMUX.captures(line) {
        return Some(Event::PublicAddress { ip: c[1].to_string() });
    }
    if let Some(c) = JOINED.captures(line) {
        return Some(Event::Joined { ip: c[1].to_string() });
    }
    if line.contains("[FLog::SingleSurfaceApp] leaveUGCGameInternal") || line.contains("[FLog::Network] Time to disconnect replication data") {
        return Some(Event::Left);
    }
    if line.contains("GameJoinUtil::joinGamePostPrivateServer") {
        return Some(Event::PrivateServer);
    }
    if line.contains("GameJoinUtil::initiateTeleportToReservedServer") {
        return Some(Event::ReservedServer);
    }
    None
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ServerKind {
    Public,
    Private,
    Reserved,
}

/// A detected stay in one game server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameVisit {
    pub id: String,
    pub place_id: u64,
    pub universe_id: Option<u64>,
    pub job_id: String,
    pub server_kind: ServerKind,
    pub started_at: chrono::DateTime<chrono::Local>,
    pub ended_at: Option<chrono::DateTime<chrono::Local>>,
    pub name: Option<String>,
    pub creator: Option<String>,
    pub icon_url: Option<String>,
    /// e.g. "Ashburn, Virginia, US". The server's IP address is not stored.
    pub location: Option<String>,
}

impl GameVisit {
    pub fn can_rejoin(&self) -> bool {
        self.server_kind == ServerKind::Public
    }
    /// Deep link that joins this exact server (public servers only).
    pub fn rejoin_link(&self) -> String {
        format!("roblox://experiences/start?placeId={}&gameInstanceId={}", self.place_id, self.job_id)
    }
    pub fn play_link(&self) -> String {
        format!("roblox://experiences/start?placeId={}", self.place_id)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    /// Joined a server; the IP is only kept in memory to look up the location.
    Joined(GameVisit, Option<String>),
    Left(GameVisit),
}

#[derive(Debug, Default)]
struct Pending {
    job_id: String,
    place_id: u64,
    universe_id: Option<u64>,
    public_ip: Option<String>,
    kind: Option<ServerKind>,
}

#[derive(Debug, Default)]
pub struct Tracker {
    pending: Option<Pending>,
    next_kind: Option<ServerKind>,
    current: Option<GameVisit>,
}

impl Tracker {
    pub fn current(&self) -> Option<&GameVisit> {
        self.current.as_ref()
    }

    pub fn feed(&mut self, ev: Event) -> Vec<Change> {
        let mut out = vec![];
        match ev {
            Event::PrivateServer => self.next_kind = Some(ServerKind::Private),
            Event::ReservedServer => self.next_kind = Some(ServerKind::Reserved),
            Event::Joining { job_id, place_id } => {
                if let Some(c) = self.leave() {
                    out.push(c); // teleported without a clean leave
                }
                self.pending = Some(Pending { job_id, place_id, kind: self.next_kind.take(), ..Default::default() });
            }
            Event::Universe { place_id, universe_id } => {
                if let Some(p) = self.pending.as_mut().filter(|p| p.place_id == place_id) {
                    p.universe_id = Some(universe_id);
                }
            }
            Event::PublicAddress { ip } => {
                if let Some(p) = self.pending.as_mut() {
                    p.public_ip = Some(ip);
                }
            }
            Event::Joined { ip } => {
                if let Some(p) = self.pending.take() {
                    let visit = GameVisit {
                        id: uuid::Uuid::new_v4().to_string(),
                        place_id: p.place_id,
                        universe_id: p.universe_id,
                        job_id: p.job_id,
                        server_kind: p.kind.unwrap_or(ServerKind::Public),
                        started_at: chrono::Local::now(),
                        ended_at: None,
                        name: None,
                        creator: None,
                        icon_url: None,
                        location: None,
                    };
                    self.current = Some(visit.clone());
                    out.push(Change::Joined(visit, p.public_ip.or(Some(ip))));
                }
            }
            Event::Left => {
                self.pending = None;
                if let Some(c) = self.leave() {
                    out.push(c);
                }
            }
        }
        out
    }

    /// End the current visit (e.g. when Roblox closes).
    pub fn leave(&mut self) -> Option<Change> {
        self.current.take().map(|mut v| {
            v.ended_at = Some(chrono::Local::now());
            Change::Left(v)
        })
    }
}

/// `%LOCALAPPDATA%\Roblox\logs`
pub fn default_logs_dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|l| PathBuf::from(l).join("Roblox").join("logs"))
}

/// The newest player log created at or after `since`.
pub fn find_log(dir: &Path, since: SystemTime) -> Option<PathBuf> {
    let mut best: Option<(SystemTime, PathBuf)> = None;
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if !name.contains("_Player_") || !name.ends_with("_last.log") || name.contains("CrashHandler") {
            continue;
        }
        let Ok(meta) = e.metadata() else { continue };
        let t = meta.created().or_else(|_| meta.modified()).unwrap_or(SystemTime::UNIX_EPOCH);
        if t + Duration::from_secs(10) < since {
            continue;
        }
        if best.as_ref().map(|(bt, _)| t > *bt).unwrap_or(true) {
            best = Some((t, e.path()));
        }
    }
    best.map(|(_, p)| p)
}

/// Follow a log file until `stop` is set, calling `on_change` for each change.
pub fn follow(path: &Path, stop: Arc<AtomicBool>, mut on_change: impl FnMut(Change)) {
    let mut tracker = Tracker::default();
    let mut offset = 0u64;
    let mut carry = String::new();
    loop {
        if let Ok(mut f) = std::fs::File::open(path) {
            let len = f.metadata().map(|m| m.len()).unwrap_or(0);
            if len < offset {
                offset = 0; // file was replaced
            }
            if len > offset && f.seek(SeekFrom::Start(offset)).is_ok() {
                let mut buf = Vec::with_capacity((len - offset) as usize);
                if f.take(len - offset).read_to_end(&mut buf).is_ok() {
                    offset += buf.len() as u64;
                    carry.push_str(&String::from_utf8_lossy(&buf));
                    while let Some(i) = carry.find('\n') {
                        let line: String = carry.drain(..=i).collect();
                        if let Some(ev) = parse_line(&line) {
                            for c in tracker.feed(ev) {
                                on_change(c);
                            }
                        }
                    }
                }
            }
        }
        if stop.load(Ordering::SeqCst) {
            if let Some(c) = tracker.leave() {
                on_change(c);
            }
            return;
        }
        std::thread::sleep(Duration::from_millis(750));
    }
}

#[derive(Debug, Clone, Default)]
pub struct GameDetails {
    pub universe_id: Option<u64>,
    pub name: Option<String>,
    pub creator: Option<String>,
    pub icon_url: Option<String>,
}

/// Look up a game's name, creator and icon from Roblox's public APIs.
pub async fn fetch_details(http: &reqwest::Client, place_id: u64, universe_id: Option<u64>) -> GameDetails {
    let mut d = GameDetails { universe_id, ..Default::default() };
    if d.universe_id.is_none() {
        let url = format!("https://apis.roblox.com/universes/v1/places/{place_id}/universe");
        if let Ok(r) = http.get(url).send().await {
            if let Ok(v) = r.json::<serde_json::Value>().await {
                d.universe_id = v["universeId"].as_u64();
            }
        }
    }
    let Some(u) = d.universe_id else { return d };
    if let Ok(r) = http.get(format!("https://games.roblox.com/v1/games?universeIds={u}")).send().await {
        if let Ok(v) = r.json::<serde_json::Value>().await {
            let g = &v["data"][0];
            d.name = g["name"].as_str().map(str::to_string);
            d.creator = g["creator"]["name"].as_str().map(str::to_string);
        }
    }
    let icon = format!("https://thumbnails.roblox.com/v1/games/icons?universeIds={u}&returnPolicy=PlaceHolder&size=512x512&format=Png&isCircular=false");
    if let Ok(r) = http.get(icon).send().await {
        if let Ok(v) = r.json::<serde_json::Value>().await {
            d.icon_url = v["data"][0]["imageUrl"].as_str().filter(|s| s.starts_with("https://")).map(str::to_string);
        }
    }
    d
}

/// City, region and country of a Roblox server, from ipinfo.io.
pub async fn fetch_location(http: &reqwest::Client, ip: &str) -> Option<String> {
    let ok = ip.split('.').count() == 4 && ip.split('.').all(|p| p.parse::<u8>().is_ok());
    if !ok {
        return None;
    }
    let v: serde_json::Value = http.get(format!("https://ipinfo.io/{ip}/json")).send().await.ok()?.json().await.ok()?;
    let parts: Vec<&str> = ["city", "region", "country"].iter().filter_map(|k| v[*k].as_str()).filter(|s| !s.is_empty()).collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Lines as written by Roblox 0.747 (user id removed).
    const LOG: &str = "\
2026-11-11T22:53:11.462Z,0.462022,1af4,6 [FLog::SingleSurfaceApp] returnToLuaApp: (stage:InitializedLuaApp).
2026-11-11T22:53:17.648Z,6.648161,84c0,6 [FLog::Output] ! Joining game 'd1afac81-7489-46bd-9b44-dea5786982d3' place 9391468976 at 10.33.9.30
2026-11-11T22:53:17.648Z,6.648161,84c0,6 [FLog::GameJoinLoadTime] Report game_join_loadtime: placeid:9391468976, join_time:0.38508813211694370437, universeid:3508322461, referral_page:Home, sid:e382ac7b-ae9a-4499-b94f-3ae513e067e1, clienttime:1794437598.24
2026-11-11T22:53:17.654Z,6.654095,07a0,7 [FLog::Network] UDMUX Address = 128.116.86.33, Port = 53646 | RCC Server Address = 10.33.9.30, Port = 53646
2026-11-11T22:53:17.961Z,6.961028,0f84,7 [FLog::Network] serverId: 128.116.86.33|53646
2026-11-11T22:53:24.241Z,13.241063,1af4,6 [FLog::SingleSurfaceApp] leaveUGCGameInternal
2026-11-11T22:53:24.855Z,13.855141,8a44,7 [FLog::Network] Time to disconnect replication data: 0.439100
2026-11-11T22:53:28.773Z,17.773228,8a44,6 [FLog::Output] ! Joining game '6a6584ef-7877-462b-bbcc-f230e884725e' place 17625359962 at 10.39.3.207
2026-11-11T22:53:28.773Z,17.773228,8a44,6 [FLog::GameJoinLoadTime] Report game_join_loadtime: placeid:17625359962, join_time:0.17372084917500529855, universeid:6035872082, referral_page:, sid:3a5c3161-233c-4eca-8349-0b67f102a0ae, clienttime:1794437609.3719
2026-11-11T22:53:28.778Z,17.778217,21e8,7 [FLog::Network] UDMUX Address = 128.116.14.33, Port = 49780 | RCC Server Address = 10.39.3.207, Port = 49780
2026-11-11T22:53:28.889Z,17.889507,21e8,7 [FLog::Network] serverId: 128.116.14.33|49780
";

    #[test]
    fn parses_real_lines() {
        let evs: Vec<Event> = LOG.lines().filter_map(parse_line).collect();
        assert_eq!(evs[0], Event::Joining { job_id: "d1afac81-7489-46bd-9b44-dea5786982d3".into(), place_id: 9391468976 });
        assert_eq!(evs[1], Event::Universe { place_id: 9391468976, universe_id: 3508322461 });
        assert_eq!(evs[2], Event::PublicAddress { ip: "128.116.86.33".into() });
        assert_eq!(evs[3], Event::Joined { ip: "128.116.86.33".into() });
        assert_eq!(evs[4], Event::Left);
        assert!(parse_line("[FLog::Output] hello").is_none());
    }

    #[test]
    fn tracks_two_visits() {
        let mut t = Tracker::default();
        let changes: Vec<Change> = LOG.lines().filter_map(parse_line).flat_map(|e| t.feed(e)).collect();
        // joined, left (two leave lines collapse to one), joined
        assert_eq!(changes.len(), 3);
        let Change::Joined(a, ip) = &changes[0] else { panic!() };
        assert_eq!(a.place_id, 9391468976);
        assert_eq!(a.universe_id, Some(3508322461));
        assert_eq!(ip.as_deref(), Some("128.116.86.33"));
        assert!(a.can_rejoin());
        assert_eq!(a.rejoin_link(), "roblox://experiences/start?placeId=9391468976&gameInstanceId=d1afac81-7489-46bd-9b44-dea5786982d3");
        assert!(matches!(&changes[1], Change::Left(v) if v.ended_at.is_some()));
        let Change::Joined(b, _) = &changes[2] else { panic!() };
        assert_eq!(b.universe_id, Some(6035872082));
        assert_eq!(t.current().unwrap().place_id, 17625359962);
        assert!(matches!(t.leave(), Some(Change::Left(_))));
    }

    #[test]
    fn private_servers_are_not_rejoinable() {
        let mut t = Tracker::default();
        t.feed(Event::PrivateServer);
        t.feed(Event::Joining { job_id: "a".repeat(36), place_id: 1 });
        let c = t.feed(Event::Joined { ip: "1.2.3.4".into() });
        let Change::Joined(v, _) = &c[0] else { panic!() };
        assert_eq!(v.server_kind, ServerKind::Private);
        assert!(!v.can_rejoin());
    }

    #[test]
    fn follows_a_growing_file_and_finds_logs() {
        use std::io::Write;
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("0.747.0_20261111T225311Z_Player_A04D2_last.log");
        std::fs::write(d.path().join("x_Player_1_CrashHandler_last.log"), "").unwrap();
        let mut f = std::fs::File::create(&p).unwrap();
        assert_eq!(find_log(d.path(), SystemTime::now() - Duration::from_secs(5)).unwrap(), p);
        let stop = Arc::new(AtomicBool::new(false));
        let got = Arc::new(std::sync::Mutex::new(vec![]));
        let (s2, g2, p2) = (stop.clone(), got.clone(), p.clone());
        let h = std::thread::spawn(move || follow(&p2, s2, |c| g2.lock().unwrap().push(c)));
        let lines: Vec<&str> = LOG.lines().collect();
        // write a line in two pieces to check partial-line handling
        let (a, b) = lines[1].split_at(30);
        write!(f, "{a}").unwrap();
        f.flush().unwrap();
        std::thread::sleep(Duration::from_millis(900));
        writeln!(f, "{b}").unwrap();
        for l in &lines[2..5] {
            writeln!(f, "{l}").unwrap();
        }
        f.flush().unwrap();
        std::thread::sleep(Duration::from_millis(1600));
        assert!(matches!(got.lock().unwrap().first(), Some(Change::Joined(v, _)) if v.place_id == 9391468976));
        stop.store(true, Ordering::SeqCst);
        h.join().unwrap();
        assert!(matches!(got.lock().unwrap().last(), Some(Change::Left(_))), "closing Roblox ends the visit");
    }
}
