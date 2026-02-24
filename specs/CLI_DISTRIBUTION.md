# CLI Distribution — AgentLang

**Date:** 2026-02-25
**Round:** 9
**Binary:** `al`

---

## Quick-Start Install (no Rust/Cargo required)

### One-liner

```bash
curl -fsSL https://raw.githubusercontent.com/mohocp/Axon/main/install.sh | sh
```

### Custom install directory

```bash
AL_INSTALL=~/.local/bin curl -fsSL https://raw.githubusercontent.com/mohocp/Axon/main/install.sh | sh
```

### Pin a specific version

```bash
AL_VERSION=v0.1.0 curl -fsSL https://raw.githubusercontent.com/mohocp/Axon/main/install.sh | sh
```

### Verify installation

```bash
al run examples/calculate.al
# Expected output: Result: 94
```

---

## Supported Platforms

| Target | OS | Arch | Archive |
|--------|----|------|---------|
| `x86_64-unknown-linux-gnu` | Linux | x86_64 | `.tar.gz` |
| `aarch64-unknown-linux-gnu` | Linux | aarch64/ARM64 | `.tar.gz` |
| `x86_64-apple-darwin` | macOS | Intel | `.tar.gz` |
| `aarch64-apple-darwin` | macOS | Apple Silicon | `.tar.gz` |

---

## Migration from `cargo run` to `al`

### Before (source-only, requires Rust toolchain)

```bash
# Build
cargo build --workspace

# Run
cargo run -p al-cli -- run examples/calculate.al
cargo run -p al-cli -- lex examples/calculate.al
cargo run -p al-cli -- parse examples/calculate.al
cargo run -p al-cli -- check examples/calculate.al
```

### After (pre-built binary)

```bash
# Install (one-time)
curl -fsSL https://raw.githubusercontent.com/mohocp/Axon/main/install.sh | sh

# Run
al run examples/calculate.al
al lex examples/calculate.al
al parse examples/calculate.al
al check examples/calculate.al
```

### Key differences

| Aspect | `cargo run` (old) | `al` (new) |
|--------|-------------------|------------|
| **Command** | `cargo run -p al-cli -- <cmd>` | `al <cmd>` |
| **Requires Rust** | Yes | No |
| **Build step** | Every invocation (unless cached) | Pre-built binary |
| **Install method** | `cargo build --release` | `install.sh` or GitHub Release download |
| **Binary name** | `al-cli` (crate default) | `al` (canonical) |

---

## CLI Command Reference

```
al <command> [file.al] [--format human|json|jsonl]

Commands:
  lex    <file>  Tokenize and print tokens
  parse  <file>  Parse and print AST summary
  check  <file>  Type-check a source file
  run    <file>  Parse, check, and execute

Options:
  --format human   Human-readable output (default)
  --format json    JSON diagnostic output
  --format jsonl   JSON Lines diagnostic output
```

---

## Release Automation

### How releases work

1. Maintainer pushes a git tag: `git tag v0.1.0 && git push origin v0.1.0`
2. GitHub Actions `release.yml` triggers on the tag
3. Four binaries are built in parallel (Linux x86_64, Linux aarch64, macOS x86_64, macOS aarch64)
4. SHA-256 checksums are generated per artifact
5. A GitHub Release is created with all binaries and checksums attached

### Release artifacts

For each version tag `vX.Y.Z`, the following artifacts are published:

```
al-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz
al-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz.sha256
al-vX.Y.Z-aarch64-unknown-linux-gnu.tar.gz
al-vX.Y.Z-aarch64-unknown-linux-gnu.tar.gz.sha256
al-vX.Y.Z-x86_64-apple-darwin.tar.gz
al-vX.Y.Z-x86_64-apple-darwin.tar.gz.sha256
al-vX.Y.Z-aarch64-apple-darwin.tar.gz
al-vX.Y.Z-aarch64-apple-darwin.tar.gz.sha256
```

### Checksum verification

```bash
# After downloading manually:
shasum -a 256 -c al-v0.1.0-aarch64-apple-darwin.tar.gz.sha256
```

The install script verifies checksums automatically.

---

## Building from Source (developers)

For contributors who need the full Rust toolchain:

```bash
# Prerequisites
rustup install 1.75.0   # Minimum supported Rust version

# Build all crates
cargo build --workspace

# Build release binary only
cargo build --release -p al-cli

# Install to ~/.cargo/bin
cargo install --path crates/al-cli

# Run tests
cargo test --workspace
```
