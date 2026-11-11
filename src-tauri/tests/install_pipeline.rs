//! Integration test: full install pipeline against a local HTTP server that
//! imitates Roblox's deployment CDN (manifest + zip packages, Range support).

use aeolus_lib::installation::{self, deploy, detect, manifest::ClientVersion, progress::{CancelToken, Progress}, InstallJob};
use md5::Digest;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const GUID: &str = "version-00aa11bb22cc33dd";

fn zip_bytes(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, data) in files {
            z.start_file(*name, opts).unwrap();
            z.write_all(data).unwrap();
        }
        z.finish().unwrap();
    }
    buf.into_inner()
}

struct Server {
    base: String,
    files: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    /// If > 0, the next response for this many requests is cut off halfway.
    truncate_next: Arc<AtomicUsize>,
    ranged_requests: Arc<AtomicUsize>,
}

fn serve() -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let files: Arc<Mutex<HashMap<String, Vec<u8>>>> = Arc::default();
    let truncate_next = Arc::new(AtomicUsize::new(0));
    let ranged = Arc::new(AtomicUsize::new(0));
    let (f, t, r) = (files.clone(), truncate_next.clone(), ranged.clone());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut s) = stream else { continue };
            let (f, t, r) = (f.clone(), t.clone(), r.clone());
            std::thread::spawn(move || {
                let mut reader = BufReader::new(s.try_clone().unwrap());
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                let path = line.split_whitespace().nth(1).unwrap_or("/").trim_start_matches('/').to_string();
                let mut range_start = None;
                loop {
                    let mut h = String::new();
                    if reader.read_line(&mut h).unwrap() == 0 || h == "\r\n" {
                        break;
                    }
                    let lower = h.to_ascii_lowercase();
                    if let Some(v) = lower.strip_prefix("range: bytes=") {
                        range_start = v.trim().trim_end_matches('-').parse::<usize>().ok();
                    }
                }
                let body = f.lock().unwrap().get(&path).cloned();
                let Some(body) = body else {
                    let _ = s.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                    return;
                };
                let (status, slice) = match range_start {
                    Some(st) if st < body.len() => {
                        r.fetch_add(1, Ordering::SeqCst);
                        ("206 Partial Content", &body[st..])
                    }
                    _ => ("200 OK", &body[..]),
                };
                let header = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", slice.len());
                let _ = s.write_all(header.as_bytes());
                if path.ends_with(".zip") && t.load(Ordering::SeqCst) > 0 {
                    t.fetch_sub(1, Ordering::SeqCst);
                    let _ = s.write_all(&slice[..slice.len() / 2]);
                    let _ = s.shutdown(std::net::Shutdown::Both);
                    return;
                }
                let _ = s.write_all(slice);
            });
        }
    });
    Server { base, files, truncate_next, ranged_requests: ranged }
}

fn publish(server: &Server, packages: &[(&str, Vec<u8>)], corrupt_md5_for: Option<&str>) {
    let mut manifest = String::from("v0\n");
    let mut files = server.files.lock().unwrap();
    for (name, data) in packages {
        let md5 = if Some(*name) == corrupt_md5_for { "0".repeat(32) } else { hex::encode(md5::Md5::digest(data)) };
        manifest.push_str(&format!("{name}\n{md5}\n{}\n{}\n", data.len(), data.len() * 2));
        files.insert(format!("{GUID}-{name}"), data.clone());
    }
    files.insert(format!("{GUID}-rbxPkgManifest.txt"), manifest.into_bytes());
}

fn packages() -> Vec<(&'static str, Vec<u8>)> {
    let big: Vec<u8> = (0..400_000u32).map(|i| (i % 251) as u8).collect();
    vec![
        ("RobloxApp.zip", zip_bytes(&[("RobloxPlayerBeta.exe", b"MZ fake player"), ("RobloxPlayerBeta.dll", &big)])),
        ("content-fonts.zip", zip_bytes(&[("arial.ttf", b"font"), ("sub/dir.txt", b"nested")])),
        ("shaders.zip", zip_bytes(&[("shaders.pack", b"shader")])),
        ("RobloxPlayerInstaller.exe", b"installer is not part of layout".to_vec()),
        ("mystery-new-package.zip", zip_bytes(&[("x.bin", b"x")])),
    ]
}

fn job<'a>(http: &'a reqwest::Client, server: &Server, root: &std::path::Path, events: Arc<Mutex<Vec<Progress>>>, cancel: CancelToken) -> InstallJob<'a> {
    InstallJob {
        http,
        hosts: vec![format!("{}/missing-host", server.base), server.base.clone()],
        root: root.join("Versions"),
        cache_dir: root.join("downloads"),
        keep_cache: false,
        cancel,
        sink: Arc::new(move |p| events.lock().unwrap().push(p)),
        operation: "test".into(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn full_install_verify_repair_and_remove() {
    let server = serve();
    publish(&server, &packages(), None);
    let dir = tempfile::tempdir().unwrap();
    let http = reqwest::Client::new();
    let events = Arc::new(Mutex::new(vec![]));
    let target = ClientVersion { version: "0.700.0.1".into(), version_guid: GUID.into() };

    // First attempt: the big package is cut off mid-transfer, so this run fails
    // with a resumable partial file left behind.
    server.truncate_next.store(1, Ordering::SeqCst);
    let j = job(&http, &server, dir.path(), events.clone(), CancelToken::new());
    let first = installation::install(&j, &target, None).await;
    assert!(first.is_err(), "interrupted download must not report success");
    assert!(!dir.path().join("Versions").join(GUID).exists(), "nothing is installed after a failure");

    // Second attempt resumes with a Range request and completes.
    let outcome = installation::install(&j, &target, None).await.expect("install");
    assert!(server.ranged_requests.load(Ordering::SeqCst) >= 1, "partial download was resumed");
    let inst = std::path::PathBuf::from(&outcome.installed.path);
    assert!(inst.join("RobloxPlayerBeta.exe").exists());
    assert!(inst.join("content/fonts/sub/dir.txt").exists());
    assert!(inst.join("shaders/shaders.pack").exists());
    assert!(inst.join("AppSettings.xml").exists());
    assert!(!inst.join("x.bin").exists(), "unknown packages are skipped");
    assert_eq!(outcome.warnings.len(), 1);
    assert_eq!(outcome.installed.file_count, 5);
    {
        let ev = events.lock().unwrap();
        assert!(ev.iter().any(|p| p.stage == "Downloading" && p.total.is_some()));
        assert!(ev.iter().any(|p| p.stage == "Extracting"));
        assert!(ev.last().unwrap().done);
    }
    // cache cleaned (keep_cache = false)
    let leftover = std::fs::read_dir(dir.path().join("downloads")).unwrap().count();
    assert_eq!(leftover, 0);

    // detection sees it as a valid managed install
    let roots = vec![detect::ScanRoot { source: "aeolus", label: "Aeolus", path: dir.path().join("Versions"), managed: true }];
    let det = detect::scan(&roots);
    assert_eq!(det.installs.len(), 1);
    assert!(det.installs[0].valid, "{:?}", det.installs[0].issues);
    assert_eq!(det.installs[0].version.as_deref(), Some("0.700.0.1"));

    // verify: intact
    let c = CancelToken::new();
    let r = deploy::verify(&inst, true, &c, &|_, _| {}).unwrap();
    assert!(r.ok());
    // tamper: same size different content -> only full verify catches it; delete one file
    std::fs::write(inst.join("shaders/shaders.pack"), b"SHADER").unwrap();
    std::fs::remove_file(inst.join("content/fonts/arial.ttf")).unwrap();
    let quick = deploy::verify(&inst, false, &c, &|_, _| {}).unwrap();
    assert_eq!(quick.missing, vec!["content/fonts/arial.ttf".to_string()]);
    assert!(quick.modified.is_empty());
    let full = deploy::verify(&inst, true, &c, &|_, _| {}).unwrap();
    assert_eq!(full.modified, vec!["shaders/shaders.pack".to_string()]);

    // repair = reinstall same version; user-applied ClientSettings is not part of the index
    std::fs::create_dir_all(inst.join("ClientSettings")).unwrap();
    let repaired = installation::install(&j, &target, Some(outcome.installed.clone())).await.unwrap();
    let inst2 = std::path::PathBuf::from(&repaired.installed.path);
    assert!(deploy::verify(&inst2, true, &c, &|_, _| {}).unwrap().ok());

    // remove: refuses folders Aeolus did not create
    let foreign = dir.path().join("Versions").join("version-ffff0000");
    std::fs::create_dir_all(&foreign).unwrap();
    assert!(installation::remove_managed(&foreign).is_err());
    assert!(foreign.exists());
    installation::remove_managed(&inst2).unwrap();
    assert!(!inst2.exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn checksum_mismatch_is_reported_not_installed() {
    let server = serve();
    publish(&server, &packages(), Some("shaders.zip"));
    let dir = tempfile::tempdir().unwrap();
    let http = reqwest::Client::new();
    let j = job(&http, &server, dir.path(), Arc::default(), CancelToken::new());
    let target = ClientVersion { version: "1".into(), version_guid: GUID.into() };
    let err = installation::install(&j, &target, None).await.unwrap_err();
    assert_eq!(err.kind(), "integrity", "{err}");
    assert!(!dir.path().join("Versions").join(GUID).exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn cancellation_stops_without_installing() {
    let server = serve();
    publish(&server, &packages(), None);
    let dir = tempfile::tempdir().unwrap();
    let http = reqwest::Client::new();
    let cancel = CancelToken::new();
    cancel.cancel();
    let j = job(&http, &server, dir.path(), Arc::default(), cancel);
    let target = ClientVersion { version: "1".into(), version_guid: GUID.into() };
    let err = installation::install(&j, &target, None).await.unwrap_err();
    assert_eq!(err.kind(), "cancelled");
}

#[tokio::test(flavor = "multi_thread")]
async fn readonly_or_invalid_root_fails_early() {
    let server = serve();
    publish(&server, &packages(), None);
    let dir = tempfile::tempdir().unwrap();
    // a *file* where the versions directory should be
    std::fs::write(dir.path().join("Versions"), b"not a dir").unwrap();
    let http = reqwest::Client::new();
    let j = job(&http, &server, dir.path(), Arc::default(), CancelToken::new());
    let target = ClientVersion { version: "1".into(), version_guid: GUID.into() };
    assert!(installation::install(&j, &target, None).await.is_err());
}
