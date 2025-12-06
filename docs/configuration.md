# Configuration Guide

Velo Hub supports flexible configuration through multiple sources with clear precedence rules.

## Configuration Precedence

Configuration values are loaded in the following order (highest to lowest priority):

1. **Environment Variables** - Override any configuration value
2. **Configuration File** - `~/.velo-hub/config.toml`
3. **Default Values** - Built-in sensible defaults

## Configuration File

### Location

The configuration file should be placed at:
- **macOS/Linux**: `~/.velo-hub/config.toml`
- **Windows**: `%USERPROFILE%\.velo-hub\config.toml`

### Format

The configuration file uses TOML format. See `config.example.toml` in the project root for a complete example.

### Creating Your Configuration

1. Copy the example configuration:
   ```bash
   mkdir -p ~/.velo-hub
   cp config.example.toml ~/.velo-hub/config.toml
   ```

2. Edit the configuration file with your preferred settings:
   ```bash
   nano ~/.velo-hub/config.toml
   ```

### Configuration Sections

#### Server Configuration

Controls the HTTP server settings:

```toml
[server]
host = "127.0.0.1"  # Bind address (use "0.0.0.0" for all interfaces)
port = 8080          # Port number
enable_tls = false   # Enable HTTPS (requires certificate setup)
```

#### Database Configuration

Specifies the SQLite database location:

```toml
[database]
path = "~/.velo-hub/data.db"  # Supports tilde (~) expansion
```

#### Circuit Breaker Configuration

Controls the circuit breaker behavior for provider protection:

```toml
[circuit_breaker]
failure_threshold = 0.5           # Error rate threshold (0.0-1.0)
window_duration_secs = 60         # Time window for error rate calculation
cooldown_duration_secs = 30       # Cooldown before retry
half_open_max_requests = 3        # Test requests in half-open state
```

#### Session Configuration

Manages session lifecycle:

```toml
[session]
ttl_minutes = 5                   # Session expiration time
cleanup_interval_secs = 60        # Cleanup task interval
```

#### Logging Configuration

Controls application logging:

```toml
[logging]
level = "info"                    # Log level: trace, debug, info, warn, error
file = "~/.velo-hub/logs/app.log"  # Log file path (optional)
```

## Environment Variables

Override any configuration value using environment variables with the `VELO_HUB_` prefix.

### Variable Naming Convention

Convert the TOML path to uppercase and join with underscores:
- `server.host` → `VELO_HUB_SERVER_HOST`
- `circuit_breaker.failure_threshold` → `VELO_HUB_CIRCUIT_BREAKER_FAILURE_THRESHOLD`

### Examples

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

### Using Environment Variables in Docker

```dockerfile
ENV VELO_HUB_SERVER_HOST=0.0.0.0
ENV VELO_HUB_SERVER_PORT=8080
ENV VELO_HUB_DATABASE_PATH=/data/velo-hub.db
ENV VELO_HUB_LOGGING_LEVEL=info
```

## Default Configuration

If no configuration file exists and no environment variables are set, the application uses these defaults:

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

## Configuration Management via UI

The application also provides UI-based configuration management:

1. Open the Settings page in the application
2. Modify configuration values
3. Click "Save" to persist changes to `~/.velo-hub/config.toml`

Note: Environment variables will still override UI-saved values.

## Troubleshooting

### Configuration Not Loading

1. Check file permissions:
   ```bash
   ls -la ~/.velo-hub/config.toml
   ```

2. Verify TOML syntax:
   ```bash
   cat ~/.velo-hub/config.toml
   ```

3. Check application logs for configuration errors:
   ```bash
   tail -f ~/.velo-hub/logs/app.log
   ```

### Environment Variables Not Working

1. Verify the variable is set:
   ```bash
   echo $VELO_HUB_SERVER_PORT
   ```

2. Ensure the variable name follows the correct format (uppercase with underscores)

3. Restart the application after setting environment variables

### Path Expansion Issues

- Use `~/` for home directory expansion
- Use absolute paths for system-wide locations
- Relative paths are resolved from the application data directory

## Best Practices

1. **Development**: Use the configuration file for local development
2. **Production**: Use environment variables for deployment
3. **Secrets**: Never commit configuration files with sensitive data to version control
4. **Backup**: Regularly backup your configuration file
5. **Documentation**: Document any custom configuration values for your team

## Security Considerations

- API keys are encrypted and stored separately in the database
- Configuration files should have restricted permissions (600)
- Use environment variables for sensitive values in production
- The configuration file does not contain API keys or secrets
