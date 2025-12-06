# Velo Hub

Velo Hub is a Tauri-based desktop application that provides local LLM API aggregation and intelligent routing services. This system acts as a proxy layer between clients and multiple LLM providers, implementing intelligent load balancing, automatic retry, circuit breaker protection, rate limiting, and real-time monitoring.

## Tech Stack

### Frontend
- React 18 + TypeScript
- shadcn/ui (UI components)
- TanStack Query (data fetching)
- Zustand (state management)
- React Router (routing)
- Tailwind CSS (styling)
- Bun (package manager)

### Backend
- Rust + Tauri 2.0
- Axum (HTTP server)
- SQLx (database)
- SQLite (embedded database)
- Tokio (async runtime)

## Project Structure

```
.
├── src/                    # Frontend React code
│   ├── lib/               # Utility functions
│   └── index.css          # Global styles with Tailwind
├── src-tauri/             # Rust backend code
│   └── src/
│       ├── commands.rs    # Tauri commands
│       ├── config.rs      # Configuration
│       ├── crypto.rs      # Encryption service
│       ├── db.rs          # Database layer
│       ├── error.rs       # Error types
│       ├── http_server.rs # HTTP server
│       ├── limiter.rs     # Rate limiter
│       ├── models.rs      # Data models
│       ├── provider/      # Provider adapters
│       ├── router/        # Request routing
│       └── session.rs     # Session management
├── .prettierrc            # Prettier config
├── eslint.config.js       # ESLint config
├── tailwind.config.js     # Tailwind config
└── package.json           # Dependencies

```

## Development Setup

### Prerequisites
- [Bun](https://bun.sh/) - JavaScript runtime and package manager
- [Rust](https://www.rust-lang.org/) - Rust toolchain
- [Tauri Prerequisites](https://tauri.app/v2/guides/prerequisites/) - Platform-specific dependencies

### Installation

1. Install frontend dependencies:
```bash
bun install
```

2. Check Rust compilation:
```bash
cargo check --manifest-path=src-tauri/Cargo.toml
```

### Development

Run the development server:
```bash
bun run tauri dev
```

### Build

Build for production:
```bash
bun run tauri build
```

## Code Quality

### Linting
```bash
bun run lint
```

### Formatting
```bash
bun run format
```

## Configuration

The application supports configuration through multiple sources with the following precedence (highest to lowest):

1. **Environment Variables** - Override any configuration value
2. **Configuration File** - `~/.velo-hub/config.toml`
3. **Default Values** - Built-in defaults

### Configuration File

Create a configuration file at `~/.velo-hub/config.toml`. See `config.example.toml` for a complete example.

```toml
[server]
host = "127.0.0.1"
port = 8080
enable_tls = false

[database]
path = "~/.velo-hub/data.db"

[circuit_breaker]
failure_threshold = 0.5
window_duration_secs = 60
cooldown_duration_secs = 30
half_open_max_requests = 3

[session]
ttl_minutes = 5
cleanup_interval_secs = 60

[logging]
level = "info"
file = "~/.velo-hub/logs/app.log"
```

### Environment Variables

Override configuration values using environment variables with the `VELO_HUB_` prefix:

```bash
# Server configuration
export VELO_HUB_SERVER_HOST="0.0.0.0"
export VELO_HUB_SERVER_PORT="9000"
export VELO_HUB_SERVER_ENABLE_TLS="true"

# Database configuration
export VELO_HUB_DATABASE_PATH="/custom/path/data.db"

# Circuit breaker configuration
export VELO_HUB_CIRCUIT_BREAKER_FAILURE_THRESHOLD="0.6"
export VELO_HUB_CIRCUIT_BREAKER_WINDOW_DURATION_SECS="120"
export VELO_HUB_CIRCUIT_BREAKER_COOLDOWN_DURATION_SECS="60"
export VELO_HUB_CIRCUIT_BREAKER_HALF_OPEN_MAX_REQUESTS="5"

# Session configuration
export VELO_HUB_SESSION_TTL_MINUTES="10"
export VELO_HUB_SESSION_CLEANUP_INTERVAL_SECS="120"

# Logging configuration
export VELO_HUB_LOGGING_LEVEL="debug"
export VELO_HUB_LOGGING_FILE="/var/log/velo-hub.log"
```

## Features (Planned)

- ✅ Project infrastructure setup
- ✅ Configuration management with TOML and environment variables
- ⏳ Provider management (Anthropic, OpenAI, Gemini, Custom)
- ⏳ Intelligent load balancing and routing
- ⏳ Rate limiting and concurrency control
- ⏳ Circuit breaker protection
- ⏳ Session management
- ⏳ Real-time monitoring dashboard
- ⏳ OpenAI-compatible API
- ⏳ Model pricing management
- ⏳ Proxy configuration
- ⏳ Configuration import/export

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## License

MIT
