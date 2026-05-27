use criterion::{black_box, criterion_group, criterion_main, Criterion};
use flux::config::Config;
use flux::search::SearchEngine;
use flux::store::{ShellEvent, Store};
use flux::miner::{Session, WorkflowDag};

fn bench_store_ingest(c: &mut Criterion) {
    c.bench_function("store_ingest_10k", |b| {
        b.iter(|| {
            let mut store = Store::default();
            for i in 0..10_000 {
                let cmd = format!("command_{}", i);
                let ev = ShellEvent::new(&cmd);
                store.ingest(black_box(&ev));
            }
        });
    });
}

fn bench_search(c: &mut Criterion) {
    let mut engine = SearchEngine::new(&Config::default());
    let mut store = Store::default();
    
    // Populate with 50,000 realistic-looking commands
    for i in 0..50_000 {
        let cmd = format!("git commit -m 'feature/test-{}' --no-verify", i);
        let ev = ShellEvent::new(&cmd);
        store.ingest(&ev);
    }
    
    // Add some noise
    for i in 0..10_000 {
        let cmd = format!("docker run -d nginx:{}", i);
        let ev = ShellEvent::new(&cmd);
        store.ingest(&ev);
    }

    for rec in store.all_sorted() {
        engine.index(rec);
    }

    c.bench_function("search_prefix_10k", |b| {
        b.iter(|| {
            // A prefix that matches thousands of commands
            engine.search(black_box("git commit"), black_box(10))
        });
    });

    c.bench_function("search_fuzzy_complex", |b| {
        b.iter(|| {
            // A complex query that triggers BM25 and fuzzy fallback
            engine.search(black_box("gti commt feature"), black_box(10))
        });
    });
}

fn bench_workflow_miner(c: &mut Criterion) {
    let mut dag = WorkflowDag::new();
    let mut sessions = Vec::new();

    // Create 1000 sessions with 5 commands each
    for _i in 0..1000 {
        let events = vec![
            ShellEvent::new("git status"),
            ShellEvent::new("git add ."),
            ShellEvent::new("git commit -m 'test'"),
            ShellEvent::new("git push origin main"),
            ShellEvent::new("npm run deploy"),
        ];
        let mut session = Session { id: format!("sess_{}", _i), events: vec![] };
        for ev in events {
            session.events.push(ev);
        }
        sessions.push(session);
    }

    c.bench_function("miner_ingest_sessions", |b| {
        b.iter(|| {
            let mut d = WorkflowDag::new();
            d.ingest(black_box(&sessions));
        });
    });

    dag.ingest(&sessions);

    c.bench_function("miner_predict_next", |b| {
        b.iter(|| {
            dag.predict(black_box("git commit -m 'test'"), black_box(5))
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10).measurement_time(std::time::Duration::from_secs(2));
    targets = bench_store_ingest, bench_search, bench_workflow_miner
}
criterion_main!(benches);
