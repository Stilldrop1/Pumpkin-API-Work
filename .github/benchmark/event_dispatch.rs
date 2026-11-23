//! Measures `PluginManager::fire` overhead with native handlers on a cheap event.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use std::sync::{Arc, Mutex};

use criterion::{Criterion, criterion_group, criterion_main};
use pumpkin_config::{AdvancedConfiguration, BasicConfiguration, TelemetryConfig};
use pumpkin_core::{
    data::VanillaData,
    plugin::{
        BoxFuture, DynEventHandler, EventPriority, EventRoHandler, EventRwHandler, QueuedEvent,
        events::server::ServerTickStartEvent,
    },
    server::Server,
};
use pumpkin_util::PermissionLvl;

struct Edit;
impl EventRwHandler<ServerTickStartEvent> for Edit {
    fn handle<'a>(
        &'a self,
        _server: &'a Arc<Server>,
        event: &'a mut ServerTickStartEvent,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            event.tick = std::hint::black_box(event.tick + 1);
        })
    }
}

struct Read;
impl EventRoHandler<ServerTickStartEvent> for Read {
    fn handle<'a>(
        &'a self,
        _server: &'a Arc<Server>,
        event: &'a ServerTickStartEvent,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            std::hint::black_box(event.tick);
        })
    }
}

/// Keeps its handle alive past the handler, like a Wasm call still in flight.
struct Keeper(Mutex<Option<QueuedEvent>>, EventPriority);
impl DynEventHandler for Keeper {
    fn handle_dyn<'a>(&'a self, _server: &'a Arc<Server>, event: QueuedEvent) -> BoxFuture<'a, ()> {
        *self.0.lock().unwrap() = Some(event);
        Box::pin(async {})
    }
    fn is_blocking(&self) -> bool {
        true
    }
    fn get_priority(&self) -> &EventPriority {
        &self.1
    }
}

fn server(rt: &tokio::runtime::Runtime, dir: &tempfile::TempDir) -> Arc<Server> {
    let basic = BasicConfiguration {
        default_level_name: dir.path().to_string_lossy().into_owned(),
        allow_nether: false,
        allow_end: false,
        allow_chat_reports: false,
        op_permission_level: PermissionLvl::Four,
        ..Default::default()
    };
    let mut advanced = AdvancedConfiguration::default();
    advanced.networking.java.online_mode = false;
    advanced.networking.bedrock.enabled = false;
    advanced.update_checker.enabled = false;
    advanced.player_data.save_player_data = false;
    rt.block_on(Server::new(
        basic,
        advanced,
        TelemetryConfig::default(),
        VanillaData::default(),
        Vec::new(),
    ))
    .expect("server")
}

fn bench(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
    let mut group = c.benchmark_group("fire");
    let cases: [(&str, usize, usize, bool); 4] = [
        ("1_blocking", 1, 0, false),
        ("5_blocking", 5, 0, false),
        ("5_blocking_5_nonblocking", 5, 5, false),
        ("5_blocking_handle_kept", 4, 0, true),
    ];
    for (name, blocking, nonblocking, keep) in cases {
        let dir = tempfile::tempdir().unwrap();
        let server = server(&rt, &dir);
        let pm = &server.plugin_manager;
        for _ in 0..blocking {
            pm.register_blocking::<ServerTickStartEvent, _>(Arc::new(Edit), EventPriority::Normal, None);
        }
        for _ in 0..nonblocking {
            pm.register_nonblocking::<ServerTickStartEvent, _>(Arc::new(Read), EventPriority::Normal, None);
        }
        if keep {
            let keeper: Arc<dyn DynEventHandler> =
                Arc::new(Keeper(Mutex::new(None), EventPriority::Normal));
            pm.register_dyn("ServerTickStartEvent", &keeper);
        }
        group.bench_function(name, |b| {
            b.to_async(&rt).iter(|| async {
                let event = pm.fire(&server, ServerTickStartEvent::new(1)).await;
                std::hint::black_box(event.tick)
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
