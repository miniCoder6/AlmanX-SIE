use flux::config::Config;
use crossbeam_channel::{bounded, TrySendError};
use flux::store::{ShellEvent, Store, Wal};
use tokio::io::{AsyncBufReadExt, BufReader};
#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};
use tracing::{error, info, warn};

const CAP: usize = 1_024;
const SNAPSHOT_EVERY: u64 = 500;

#[cfg(windows)]
fn main() {
    tracing_subscriber::fmt::init();
    warn!("flux-daemon is not supported on Windows yet. The CLI will write directly to the database.");
}

#[cfg(unix)]
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let cfg = Config::load();
    cfg.ensure_dirs();
    if std::path::Path::new(&cfg.socket_path).exists() {
        let _ = std::fs::remove_file(&cfg.socket_path);
    }
    let listener = UnixListener::bind(&cfg.socket_path).expect("bind socket");
    info!("flux-daemon listening on {}", cfg.socket_path);
    let (tx, rx) = bounded::<ShellEvent>(CAP);
    let cfg2 = cfg.clone();
    std::thread::spawn(move || worker(cfg2, rx));
    loop {
        match listener.accept().await {
            Ok((stream, _)) => { let tx2 = tx.clone(); tokio::spawn(handle(stream, tx2)); }
            Err(e) => error!("accept: {}", e),
        }
    }
}

#[cfg(unix)]
async fn handle(stream: UnixStream, tx: crossbeam_channel::Sender<ShellEvent>) {
    let mut lines = BufReader::new(stream).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let line = line.trim().to_string();
        if line.is_empty() { continue; }
        match serde_json::from_str::<ShellEvent>(&line) {
            Ok(ev) => { if let Err(TrySendError::Full(_)) = tx.try_send(ev) { warn!("channel full, dropping"); } }
            Err(e) => warn!("parse: {}", e),
        }
    }
}

fn worker(cfg: Config, rx: crossbeam_channel::Receiver<ShellEvent>) {
    let mut wal = Wal::open(&cfg.wal_path(), cfg.max_wal_events).expect("open WAL");
    let mut store = Store::load(&cfg.store_path());
    wal.replay(|ev| store.ingest(&ev));
    info!("recovered {} commands", store.index.len());
    let mut n: u64 = 0;
    for ev in rx {
        wal.append(&ev);
        store.ingest(&ev);
        n += 1;
        if n % SNAPSHOT_EVERY == 0 {
            store.save(&cfg.store_path());
            if wal.needs_compaction() { wal.compact(); info!("WAL compacted"); }
        }
    }
    store.save(&cfg.store_path());
}
