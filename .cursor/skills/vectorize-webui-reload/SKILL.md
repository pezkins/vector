---
name: vectorize-webui-reload
description: Correctly rebuild and reload the Vectorize WebUI to ensure latest code is running. Use when UI changes aren't appearing, browser shows stale code, after modifying any ui/ files, or when debugging CSS/layout issues.
---

# Vectorize WebUI Reload Process

The Vectorize UI is a Leptos/WASM application that gets **embedded into the CLI binary**. This means UI changes require rebuilding both the UI and CLI.

## The Full Reload Process

**CRITICAL**: Follow ALL steps in order. Skipping steps will result in stale code running.

### Step 1: Kill Running Processes

```bash
pkill -f "vectorize" 2>/dev/null
pkill -f "vector" 2>/dev/null
sleep 1
```

### Step 2: Clean and Rebuild UI (WASM)

```bash
cd /Users/pezkins/github/vectorize/ui
rm -rf dist                          # Remove old build artifacts
env -u NO_COLOR trunk build --release 2>&1 | tail -5
```

**Note**: Use `env -u NO_COLOR` to prevent trunk color flag issues.

### Step 3: Rebuild CLI (embeds UI)

```bash
cd /Users/pezkins/github/vectorize/vectorize-cli
cargo build --release 2>&1 | tail -3
```

**IMPORTANT**: The CLI uses `rust-embed` to embed UI assets from `ui/dist/`. If you skip this step, the old UI will be served.

### Step 4: Start Server

```bash
cd /Users/pezkins/github/vectorize
./target/release/vectorize --config config/demo.toml 2>&1 &
sleep 3
```

### Step 5: Hard Refresh Browser

In the Cursor IDE browser or external browser:
- **Mac**: `Cmd+Shift+R`
- **Windows/Linux**: `Ctrl+Shift+R`

Or use browser MCP tool:
```
browser_reload with ignoreCache: true
```

## Common Issues

### UI Changes Not Appearing

**Cause**: Didn't rebuild CLI after UI changes.

**Fix**: Always run `cargo build --release -p vectorize` after `trunk build --release`.

### Browser Cache Showing Old Code

**Cause**: Browser cached old WASM/JS files.

**Fix**: Hard refresh (`Cmd+Shift+R`) or clear browser cache.

### CSS Scrolling Not Working

**Key insight**: `h-full` requires every parent in the chain to have a defined height.

**Solution**: Use explicit viewport-based heights:
```rust
// Instead of relying on h-full chain:
<div class="h-full overflow-y-auto">  // MAY NOT WORK

// Use explicit calc:
<div style="height: calc(100vh - 88px);">
    <div style="flex: 1 1 0%; overflow-y: auto; min-height: 0;">
        // Scrollable content
    </div>
</div>
```

### Vector Binary Not Found After Clean

If you run `cargo clean` in the workspace root, it removes the Vector binary too.

**Fix**: Rebuild Vector:
```bash
cargo build --release -p vector
```

## One-Liner Full Rebuild

```bash
pkill -f "vectorize"; pkill -f "vector"; sleep 1 && \
cd /Users/pezkins/github/vectorize/ui && rm -rf dist && env -u NO_COLOR trunk build --release && \
cd ../vectorize-cli && cargo build --release && \
cd .. && ./target/release/vectorize --config config/demo.toml &
```

## Verify New Code is Running

1. Check the server startup logs show current timestamp
2. In browser DevTools Network tab, verify WASM files have new timestamps
3. Add a `web_sys::console::log_1(&"VERSION: X".into());` temporarily to confirm

## Why This Process Exists

```
┌─────────────────────────────────────────────────┐
│                vectorize-cli binary             │
│  ┌───────────────────────────────────────────┐  │
│  │  rust-embed: embeds ui/dist/* at compile  │  │
│  │  ┌─────────────────────────────────────┐  │  │
│  │  │  vectorize-ui.wasm (WASM binary)   │  │  │
│  │  │  vectorize-ui.js (JS glue code)    │  │  │
│  │  │  index.html, output.css            │  │  │
│  │  └─────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
```

The UI files are **baked into** the CLI binary. Changing UI source files does nothing until you:
1. Rebuild UI → creates new `ui/dist/*` files
2. Rebuild CLI → embeds new `ui/dist/*` files into binary
3. Restart server → runs new binary
4. Refresh browser → loads new WASM/JS
