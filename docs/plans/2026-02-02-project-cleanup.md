# Project Cleanup Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Refactor each app into a lib+bin layout, remove `#[path]` test imports, and keep CLI/logging behavior unchanged while ensuring `cargo make lint` passes.

**Architecture:** Each app exposes a small `lib.rs` with its CLI `Args` and `run` entrypoint plus existing modules. `main.rs` becomes a thin wrapper that parses CLI args and calls the library. Tests import the library modules instead of using `#[path]`.

**Tech Stack:** Rust, clap, tokio, axum, tracing, cargo-make.

---

### Task 1: Extract `elf-api` Library Entry

**Files:**
- Create: `apps/elf-api/src/lib.rs`
- Modify: `apps/elf-api/src/main.rs`
- Modify: `apps/elf-api/tests/http.rs`
- Modify: `apps/elf-api/src/routes.rs`
- Modify: `apps/elf-api/src/state.rs`

**Step 1: Create `lib.rs` with CLI args and run entrypoint**

```rust
use clap::Parser;
use std::net::SocketAddr;

pub mod routes;
pub mod state;

#[derive(Debug, Parser)]
#[command(
    version = elf_cli::VERSION,
    rename_all = "kebab",
    styles = elf_cli::styles(),
)]
pub struct Args {
    #[arg(long, short = 'c', value_name = "FILE")]
    pub config: std::path::PathBuf,
}

pub async fn run(args: Args) -> color_eyre::Result<()> {
    let config = elf_config::load(&args.config)?;
    init_tracing(&config)?;
    let http_addr: SocketAddr = config.service.http_bind.parse()?;
    let admin_addr: SocketAddr = config.service.admin_bind.parse()?;
    if config.security.bind_localhost_only && !http_addr.ip().is_loopback() {
        return Err(color_eyre::eyre::eyre!(
            "http_bind must be a loopback address when bind_localhost_only is true."
        ));
    }
    if !admin_addr.ip().is_loopback() {
        return Err(color_eyre::eyre::eyre!("admin_bind must be a loopback address."));
    }
    let state = state::AppState::new(config).await?;
    let app = routes::router(state.clone());
    let admin_app = routes::admin_router(state);

    let http_listener = tokio::net::TcpListener::bind(http_addr).await?;
    tracing::info!(%http_addr, "HTTP server listening.");
    let http_server = axum::serve(http_listener, app);

    let admin_listener = tokio::net::TcpListener::bind(admin_addr).await?;
    tracing::info!(%admin_addr, "Admin server listening.");
    let admin_server = axum::serve(admin_listener, admin_app);

    tokio::try_join!(http_server, admin_server)?;
    Ok(())
}

fn init_tracing(config: &elf_config::Config) -> color_eyre::Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_new(&config.service.log_level)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
    Ok(())
}
```

**Step 2: Slim `main.rs` to delegate to the library**

```rust
use clap::Parser;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let args = elf_api::Args::parse();
    elf_api::run(args).await
}
```

**Step 3: Update tests to import library modules (remove `#[path]`)**

```rust
use elf_api::{routes, state};
```

**Step 4: Ensure modules compile as library exports**
- Keep `routes.rs` and `state.rs` logic the same.
- Make any items public only as needed by tests.

**Step 5: Run focused tests**
Run: `cargo test -p elf-api --test http`
Expected: PASS (4 tests).

**Step 6: Commit**

```bash
git add apps/elf-api/src/lib.rs apps/elf-api/src/main.rs apps/elf-api/tests/http.rs apps/elf-api/src/routes.rs apps/elf-api/src/state.rs
git commit -m "refactor: move elf-api entrypoint into lib"
```

---

### Task 2: Extract `elf-worker` Library Entry

**Files:**
- Create: `apps/elf-worker/src/lib.rs`
- Modify: `apps/elf-worker/src/main.rs`
- Modify: `apps/elf-worker/src/worker.rs`

**Step 1: Create `lib.rs` with CLI args and run entrypoint**

```rust
use clap::Parser;
use tracing_subscriber::EnvFilter;

pub mod worker;

#[derive(Debug, Parser)]
#[command(
    version = elf_cli::VERSION,
    rename_all = "kebab",
    styles = elf_cli::styles(),
)]
pub struct Args {
    #[arg(long, short = 'c', value_name = "FILE")]
    pub config: std::path::PathBuf,
}

pub async fn run(args: Args) -> color_eyre::Result<()> {
    let config = elf_config::load(&args.config)?;
    let filter = EnvFilter::new(config.service.log_level.clone());
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let db = elf_storage::db::Db::connect(&config.storage.postgres).await?;
    db.ensure_schema(config.storage.qdrant.vector_dim).await?;
    let qdrant = elf_storage::qdrant::QdrantStore::new(&config.storage.qdrant)?;

    let state = worker::WorkerState {
        db,
        qdrant,
        embedding: config.providers.embedding,
    };

    worker::run_worker(state).await
}
```

**Step 2: Slim `main.rs` to delegate to the library**

```rust
use clap::Parser;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let args = elf_worker::Args::parse();
    elf_worker::run(args).await
}
```

**Step 3: Ensure `worker.rs` still references `crate::worker` via lib**
- Keep logic unchanged.

**Step 4: Run focused tests**
Run: `cargo test -p elf-worker`
Expected: PASS (0 tests).

**Step 5: Commit**

```bash
git add apps/elf-worker/src/lib.rs apps/elf-worker/src/main.rs apps/elf-worker/src/worker.rs
git commit -m "refactor: move elf-worker entrypoint into lib"
```

---

### Task 3: Extract `elf-mcp` Library Entry

**Files:**
- Create: `apps/elf-mcp/src/lib.rs`
- Modify: `apps/elf-mcp/src/main.rs`
- Modify: `apps/elf-mcp/src/server.rs`

**Step 1: Create `lib.rs` with CLI args and run entrypoint**

```rust
use clap::Parser;

pub mod server;

#[derive(Debug, Parser)]
#[command(
    version = elf_cli::VERSION,
    rename_all = "kebab",
    styles = elf_cli::styles(),
)]
pub struct Args {
    #[arg(long, short = 'c', value_name = "FILE")]
    pub config: std::path::PathBuf,
}

pub async fn run(args: Args) -> color_eyre::Result<()> {
    let config = elf_config::load(&args.config)?;
    server::serve_mcp(&config.service.mcp_bind, &config.service.http_bind).await
}
```

**Step 2: Slim `main.rs` to delegate to the library**

```rust
use clap::Parser;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let args = elf_mcp::Args::parse();
    elf_mcp::run(args).await
}
```

**Step 3: Ensure `server.rs` still compiles in library context**
- Keep logic unchanged.

**Step 4: Run focused tests**
Run: `cargo test -p elf-mcp`
Expected: PASS (1 test).

**Step 5: Commit**

```bash
git add apps/elf-mcp/src/lib.rs apps/elf-mcp/src/main.rs apps/elf-mcp/src/server.rs
git commit -m "refactor: move elf-mcp entrypoint into lib"
```

---

### Task 4: Workspace Verification

**Files:**
- Modify: None

**Step 1: Run lint**
Run: `cargo make lint`
Expected: PASS.

**Step 2: Run targeted app tests**
Run: `cargo test -p elf-api --test http`
Run: `cargo test -p elf-worker`
Run: `cargo test -p elf-mcp`
Expected: PASS.

**Step 3: Optional full test (requires DB env)**
If `ELF_PG_DSN` is available, run: `cargo test -p elf-storage --test db_smoke`
Expected: PASS.

**Step 4: Commit**

```bash
git add -A
git commit -m "chore: verify workspace after app lib refactor"
```
