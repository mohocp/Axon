# Phase 4: Agents Earn Real Capabilities — Real Backends

**Principle:** Agent-Native Coordination
**Status:** Planned
**Depends on:** Phase 3 (concurrency for parallel I/O)

---

## 1. Overview

The MVP has stub backends for I/O, HTTP, and LLM operations. Agents can't actually read files, call APIs, or invoke LLMs. This phase replaces stubs with real, pluggable backends while preserving the capability-gated security model. Every real operation remains gated by the appropriate capability.

## 2. Requirements

### 2.1 Backend Trait Architecture

- **R1.1:** Define a `Backend` trait for each module category (IO, HTTP, LLM, DB, etc.).
- **R1.2:** Runtime accepts backend configuration at startup (via CLI flags or config file).
- **R1.3:** Stub backends remain as defaults for testing; real backends are opt-in.
- **R1.4:** Backend selection per module: e.g., `--io-backend real`, `--llm-backend claude`.

### 2.2 core.io — Real Filesystem

- **R2.1:** `READ(path)` reads file contents. Requires `FILE_READ` capability.
- **R2.2:** `WRITE(path, content)` writes file contents. Requires `FILE_WRITE` capability.
- **R2.3:** `FETCH(url)` performs HTTP GET for remote resources. Requires `API_CALL` capability.
- **R2.4:** All I/O operations return `Result[T]` — real errors propagate as `FAILURE`.
- **R2.5:** Path traversal attacks prevented: paths validated against sandbox allowlist.

### 2.3 core.http — Real HTTP

- **R3.1:** `GET(url, headers?)` performs real HTTP GET. Requires `API_CALL`.
- **R3.2:** `POST(url, body, headers?)` performs real HTTP POST. Requires `API_CALL`.
- **R3.3:** `PUT(url, body, headers?)` — new operation. Requires `API_CALL`.
- **R3.4:** `DELETE(url, headers?)` — new operation. Requires `API_CALL`.
- **R3.5:** Response includes status code, headers, body.
- **R3.6:** Timeout per request (default 30s, configurable).
- **R3.7:** TLS verification enabled by default.

### 2.4 agent.llm — Real LLM Integration

- **R4.1:** `GENERATE(prompt, model?, temperature?, max_tokens?)` calls LLM API. Requires `LLM_INFER`.
- **R4.2:** `CLASSIFY(input, categories, model?)` performs classification. Requires `LLM_INFER`.
- **R4.3:** `EXTRACT(input, schema, model?)` extracts structured data. Requires `LLM_INFER`.
- **R4.4:** Results returned as `Probable[T]` with model-reported confidence (Phase 2 integration).
- **R4.5:** Provider-agnostic trait: `LlmBackend` with implementations for Claude, OpenAI, local models.
- **R4.6:** Token usage tracked per call for resource budget integration.
- **R4.7:** API key configuration via environment variables or config.

### 2.5 Database Modules

- **R5.1:** `db.sql`: QUERY, INSERT, UPDATE, DELETE, MIGRATE. Requires `DB_READ`/`DB_WRITE`.
- **R5.2:** `db.vector`: UPSERT, SIMILARITY_SEARCH, CLUSTER. Requires `DB_READ`/`DB_WRITE`.
- **R5.3:** `db.graph`: TRAVERSE, SHORTEST_PATH, PATTERN_MATCH. Requires `DB_READ`.
- **R5.4:** Connection pooling via backend configuration.
- **R5.5:** Parameterized queries only (no raw SQL string interpolation — prevent injection).

### 2.6 Remaining Modules

- **R6.1:** `queue.pubsub`: PUBLISH, SUBSCRIBE, ACK, NACK, REPLAY. Requires `QUEUE_PUBLISH`/`QUEUE_SUBSCRIBE`.
- **R6.2:** `core.math`: standard arithmetic, statistics, linear algebra operations.
- **R6.3:** `core.time`: NOW, DURATION, SCHEDULE, INTERVAL.
- **R6.4:** `core.crypto`: HASH, SIGN, VERIFY, ENCRYPT, DECRYPT. Requires `CRYPTO_SIGN`/`CRYPTO_ENCRYPT`.
- **R6.5:** `core.json`: ENCODE, DECODE, VALIDATE, PATCH.
- **R6.6:** `agent.tools`: REGISTER_TOOL, INVOKE, DISCOVER. Requires `TOOL_REGISTER`/`TOOL_INVOKE`.
- **R6.7:** `agent.planning`: PLAN, DECOMPOSE, PRIORITIZE, REPLAN.
- **R6.8:** `agent.reflection`: EVALUATE, CRITIQUE, IMPROVE, LEARN.

## 3. Architecture

### 3.1 Backend Trait Pattern

```rust
pub trait IoBackend: Send + Sync {
    fn read(&self, path: &str) -> Result<Value, RuntimeFailure>;
    fn write(&self, path: &str, content: &Value) -> Result<Value, RuntimeFailure>;
    fn fetch(&self, url: &str) -> Result<Value, RuntimeFailure>;
}

pub trait HttpBackend: Send + Sync {
    fn get(&self, url: &str, headers: &Value) -> Result<Value, RuntimeFailure>;
    fn post(&self, url: &str, body: &Value, headers: &Value) -> Result<Value, RuntimeFailure>;
    fn put(&self, url: &str, body: &Value, headers: &Value) -> Result<Value, RuntimeFailure>;
    fn delete(&self, url: &str, headers: &Value) -> Result<Value, RuntimeFailure>;
}

pub trait LlmBackend: Send + Sync {
    fn generate(&self, prompt: &str, config: &LlmConfig) -> Result<Value, RuntimeFailure>;
    fn classify(&self, input: &str, categories: &[String], config: &LlmConfig) -> Result<Value, RuntimeFailure>;
    fn extract(&self, input: &str, schema: &Value, config: &LlmConfig) -> Result<Value, RuntimeFailure>;
}
```

### 3.2 Crate Changes

**New crate: `al-backends`**
- Backend traits
- Stub implementations (migrated from `al-runtime`)
- Real implementations (feature-gated)

**`al-runtime`:**
- Interpreter accepts `BackendRegistry` with configured backends
- Stdlib dispatch delegates to backend registry
- Capability checks before backend calls

**`al-stdlib-mvp`:**
- Module registry updated with new operations (PUT, DELETE, etc.)
- Signature lock updated for new operations

**`al-cli`:**
- Backend configuration flags
- Environment variable support for API keys

## 4. Testing

### 4.1 Unit Tests — Backend Traits (`al-backends`)

| ID | Test | Description |
|----|------|-------------|
| T1.1 | `test_stub_io_read` | Stub IO backend returns placeholder for READ |
| T1.2 | `test_stub_io_write` | Stub IO backend accepts WRITE without side effects |
| T1.3 | `test_stub_http_get` | Stub HTTP backend returns placeholder response |
| T1.4 | `test_stub_llm_generate` | Stub LLM backend returns placeholder text |
| T1.5 | `test_real_io_read` | Real IO backend reads existing file correctly |
| T1.6 | `test_real_io_read_missing` | Real IO backend returns FAILURE for missing file |
| T1.7 | `test_real_io_write_read_roundtrip` | Write then read returns same content |
| T1.8 | `test_real_io_path_traversal_blocked` | Path with `..` rejected |
| T1.9 | `test_real_http_get` | Real HTTP GET against httpbin or mock server |
| T1.10 | `test_real_http_post` | Real HTTP POST with body |
| T1.11 | `test_real_http_timeout` | Request exceeding timeout returns FAILURE |

### 4.2 Unit Tests — Capability Gating (`al-runtime`)

| ID | Test | Description |
|----|------|-------------|
| T2.1 | `test_io_read_requires_file_read` | READ without FILE_READ capability → CapabilityDenied |
| T2.2 | `test_io_write_requires_file_write` | WRITE without FILE_WRITE capability → CapabilityDenied |
| T2.3 | `test_http_requires_api_call` | GET/POST without API_CALL → CapabilityDenied |
| T2.4 | `test_llm_requires_llm_infer` | GENERATE without LLM_INFER → CapabilityDenied |
| T2.5 | `test_db_read_requires_db_read` | QUERY without DB_READ → CapabilityDenied |
| T2.6 | `test_crypto_requires_crypto` | ENCRYPT without CRYPTO_ENCRYPT → CapabilityDenied |

### 4.3 Unit Tests — New Operations (`al-stdlib-mvp`)

| ID | Test | Description |
|----|------|-------------|
| T3.1 | `test_http_put_signature` | PUT operation has correct signature |
| T3.2 | `test_http_delete_signature` | DELETE operation has correct signature |
| T3.3 | `test_db_sql_signatures` | All db.sql operation signatures correct |
| T3.4 | `test_core_math_signatures` | All core.math operation signatures correct |
| T3.5 | `test_core_time_signatures` | All core.time operation signatures correct |
| T3.6 | `test_core_crypto_signatures` | All core.crypto operation signatures correct |
| T3.7 | `test_agent_tools_signatures` | All agent.tools operation signatures correct |

### 4.4 Integration Tests

| ID | Test | Description |
|----|------|-------------|
| T4.1 | `test_file_read_write_e2e` | Program reads file, transforms, writes result |
| T4.2 | `test_http_get_e2e` | Program fetches URL and processes response |
| T4.3 | `test_backend_selection` | `--io-backend stub` vs `--io-backend real` switches behavior |
| T4.4 | `test_capability_denied_e2e` | Agent without FILE_READ tries READ → runtime error |

### 4.5 Conformance Tests

| ID | Test | Description |
|----|------|-------------|
| C33 | `conformance_real_io` | Real I/O operations return correct results |
| C34 | `conformance_capability_gating` | Every real operation checks capability before execution |
| C35 | `conformance_backend_pluggable` | Backend can be swapped without code changes |
| C36 | `conformance_sql_injection_prevention` | Parameterized queries prevent SQL injection |

### 4.6 Security Tests

| ID | Test | Description |
|----|------|-------------|
| S1 | `test_path_traversal` | `READ("../../etc/passwd")` blocked |
| S2 | `test_sql_injection` | SQL with injected `'; DROP TABLE` uses parameterized query |
| S3 | `test_tls_verification` | HTTP backend rejects invalid TLS certificates |
| S4 | `test_api_key_not_logged` | API keys not present in audit trail output |

## 5. Acceptance Criteria

- [ ] Real I/O backend reads and writes files on disk
- [ ] Real HTTP backend performs actual HTTP requests
- [ ] LLM backend trait defined; at least one provider implemented (stub + one real)
- [ ] All operations capability-gated (denied without proper capability)
- [ ] Stub backends remain default; real backends opt-in via flags
- [ ] No SQL injection possible through db.sql operations
- [ ] No path traversal through core.io operations
- [ ] Signature lock updated for all new operations
- [ ] All existing tests pass with stub backends
