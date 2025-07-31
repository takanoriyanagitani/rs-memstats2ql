use core::time::Duration;

use std::io;

use tokio::sync::Mutex;

use futures_util::stream::Stream;

use tokio::sync::mpsc::Receiver;
use tokio::time::Interval;

use tokio_stream::wrappers::ReceiverStream;

use async_graphql::EmptyMutation;
use async_graphql::Object;
use async_graphql::Schema;
use async_graphql::SimpleObject;
use async_graphql::Subscription;

use sysinfo::System;

#[derive(Debug, Clone, Copy, SimpleObject)]
pub struct MemoryStat {
    pub total_memory: u64,
    pub used_memory: u64,
    pub free_memory: u64,
    pub available_memory: u64,
}

pub fn sys2tot(s: &System) -> u64 {
    s.total_memory()
}

pub fn sys2used(s: &System) -> u64 {
    s.used_memory()
}

pub fn sys2free(s: &System) -> u64 {
    s.free_memory()
}

pub fn sys2available(s: &System) -> u64 {
    s.available_memory()
}

pub fn sys2stat(s: &System) -> MemoryStat {
    MemoryStat {
        total_memory: sys2tot(s),
        used_memory: sys2used(s),
        free_memory: sys2free(s),
        available_memory: sys2available(s),
    }
}

pub fn refresh_mem(s: &mut System) {
    s.refresh_memory()
}

pub async fn stats2ch(wait: Duration) -> Receiver<MemoryStat> {
    let (tx, rx) = tokio::sync::mpsc::channel(1);

    tokio::spawn(async move {
        let mut i: Interval = tokio::time::interval(wait);
        let mut s: System = System::new();
        loop {
            i.tick().await;
            refresh_mem(&mut s);
            let m: MemoryStat = sys2stat(&s);
            let rsent: Result<_, _> = tx.send(m).await;
            if let Err(e) = rsent {
                eprintln!("{e}");
                return;
            }
        }
    });

    rx
}

#[derive(Default)]
pub struct MemSub;

#[Subscription]
impl MemSub {
    async fn memory_stats(&self, wait_ms: Option<u64>) -> impl Stream<Item = MemoryStat> {
        let wait: u64 = wait_ms.unwrap_or(1_000);
        let d: Duration = Duration::from_millis(wait);
        let recv: Receiver<MemoryStat> = stats2ch(d).await;
        ReceiverStream::new(recv)
    }
}

pub struct Query {
    sys: Mutex<System>,
}

impl Default for Query {
    fn default() -> Self {
        Self {
            sys: Mutex::new(System::new()),
        }
    }
}

#[Object]
impl Query {
    pub async fn current_stat(&self) -> Result<MemoryStat, io::Error> {
        let mut guard = self.sys.lock().await;
        let ms: &mut System = &mut guard;

        refresh_mem(ms);
        Ok(sys2stat(ms))
    }
}

pub type MemSchema = Schema<Query, EmptyMutation, MemSub>;

pub fn schema_new(q: Query, s: MemSub) -> MemSchema {
    Schema::build(q, EmptyMutation, s).finish()
}

pub fn schema_default() -> MemSchema {
    let query = Query::default();
    let mem_sub = MemSub;
    schema_new(query, mem_sub)
}
