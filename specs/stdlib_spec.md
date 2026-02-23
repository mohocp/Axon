# AgentLang Standard Library API Specification

## 1. Introduction

This document defines the normative API contracts for the AgentLang Standard Library.

### 1.1 Purpose

The standard library provides verified, composable primitives for data processing, I/O, math, text, time, crypto, JSON, HTTP, agent cognition, and integration with external systems.

### 1.2 Contract Conventions

- Generic parameters use `T`, `U`, `K`, `V`, `R`, or domain-specific aliases.
- Every operation specifies `REQUIRE`, `ENSURE`, `FAILURE`, complexity, and example syntax.
- Errors are explicit: `FAILURE(ErrorCode, message: Str, details: FailureDetails)`.
- Complexity is worst-case unless stated otherwise.
- Probabilistic outputs use `Probable[T]` with confidence query `result.confidence?`.

Canonical shared aliases:

```agentlang
TYPE FailureDetails = Map[Str, JsonValue] | NONE
TYPE Result[T] = SUCCESS(T) | FAILURE(ErrorCode, message: Str, details: FailureDetails)
TYPE RegexGroup = Str | NONE
TYPE RegexMatch = {start: UInt32, end: UInt32, text: Str, groups: List[RegexGroup]}
TYPE RegexResult = Bool | Str | List[Str] | List[RegexMatch] | NONE
TYPE Token = Str | UInt32
TYPE TokenizeResult = List[Token]
```

### 1.3 Shared Failure Codes

- `INVALID_ARGUMENT`
- `TYPE_MISMATCH`
- `CONSTRAINT_VIOLATION`
- `VALIDATION_ERROR`
- `PARSE_ERROR`
- `NOT_FOUND`
- `CONFLICT`
- `UNAUTHORIZED`
- `IO_ERROR`
- `TIMEOUT`
- `RATE_LIMITED`
- `DEPENDENCY_ERROR`
- `INTERNAL`

## 2. Core Modules

### 2.1 core.data

#### core.data.FILTER
- Type Signature: `FILTER[T](items: List[T], predicate: (T) -> Bool) -> List[T]`
- REQUIRE: `items` is `List[T]`; `predicate` total over `T`.
- ENSURE: all outputs satisfy predicate; relative order preserved; `result.length <= items.length`.
- FAILURE: `TYPE_MISMATCH`; `CONSTRAINT_VIOLATION` (non-total predicate).
- Complexity: `Time O(n)`, `Space O(k)`.
- Example:
```agentlang
active = users -> FILTER (u => u.active EQ TRUE)
```

#### core.data.MAP
- Type Signature: `MAP[T,U](items: List[T], mapper: (T) -> U) -> List[U]`
- REQUIRE: mapper total and type-compatible.
- ENSURE: length preserved; index-wise mapping correctness.
- FAILURE: `TYPE_MISMATCH`; `CONSTRAINT_VIOLATION`.
- Complexity: `Time O(n)`, `Space O(n)`.
- Example:
```agentlang
emails = users -> MAP (u => u.email)
```

#### core.data.REDUCE
- Type Signature: `REDUCE[T,A](items: List[T], seed: A, reducer: (A,T) -> A) -> A`
- REQUIRE: reducer total and accumulator-safe.
- ENSURE: equivalent to left fold from `seed`.
- FAILURE: `TYPE_MISMATCH`; `CONSTRAINT_VIOLATION`.
- Complexity: `Time O(n)`, `Space O(1)`.
- Example:
```agentlang
sum = vals -> REDUCE(seed: 0, reducer: (a, x) => a + x)
```

#### core.data.SORT
- Type Signature: `SORT[T,K](items: List[T], key: (T) -> K, order: ENUM(asc,desc)=asc) -> List[T]`
- REQUIRE: key total; `K` comparable; order valid.
- ENSURE: stable ordering by key; permutation of input.
- FAILURE: `TYPE_MISMATCH`; `INVALID_ARGUMENT`.
- Complexity: `Time O(n log n)`, `Space O(n)`.
- Example:
```agentlang
ranked = products -> SORT(key: (p => p.score), order: desc)
```

#### core.data.GROUP
- Type Signature: `GROUP[T,K](items: List[T], key: (T) -> K) -> Map[K,List[T]]`
- REQUIRE: key total; `K` hashable.
- ENSURE: each item appears exactly once under `key(item)`.
- FAILURE: `TYPE_MISMATCH`; `CONSTRAINT_VIOLATION`.
- Complexity: `Time O(n)`, `Space O(n)`.
- Example:
```agentlang
by_tier = accounts -> GROUP (a => a.tier)
```

#### core.data.TAKE
- Type Signature: `TAKE[T](items: List[T], n: UInt64) -> List[T]`
- REQUIRE: `n >= 0`.
- ENSURE: result is input prefix of length `min(n, len(items))`.
- FAILURE: `INVALID_ARGUMENT`.
- Complexity: `Time O(min(n,m))`, `Space O(min(n,m))`.
- Example:
```agentlang
top20 = sorted_scores -> TAKE(20)
```

#### core.data.SKIP
- Type Signature: `SKIP[T](items: List[T], n: UInt64) -> List[T]`
- REQUIRE: `n >= 0`.
- ENSURE: result is suffix after first `n` elements.
- FAILURE: `INVALID_ARGUMENT`.
- Complexity: `Time O(max(0,m-n))`, `Space O(max(0,m-n))`.
- Example:
```agentlang
page2 = events -> SKIP(50) -> TAKE(50)
```

#### core.data.FLATTEN
- Type Signature: `FLATTEN[T](items: List[Any], depth: UInt8=1) -> List[Any]`
- REQUIRE: nested list input; `depth >= 1`.
- ENSURE: nesting reduced by up to `depth`; encounter order preserved.
- FAILURE: `TYPE_MISMATCH`; `INVALID_ARGUMENT`.
- Complexity: `Time O(n)`, `Space O(n)`.
- Example:
```agentlang
flat = batches -> FLATTEN(depth: 2)
```

#### core.data.DISTINCT
- Type Signature: `DISTINCT[T,K](items: List[T], key: (T) -> K=identity) -> List[T]`
- REQUIRE: key type hashable.
- ENSURE: unique keys; first occurrence retained.
- FAILURE: `TYPE_MISMATCH`.
- Complexity: `Time O(n)`, `Space O(n)`.
- Example:
```agentlang
uniq = orders -> DISTINCT(key: (o => o.customer_id))
```

#### core.data.ZIP
- Type Signature: `ZIP[A,B](left: List[A], right: List[B], mode: ENUM(strict,truncate)=truncate) -> List[(A,B)]`
- REQUIRE: mode valid; lists typed.
- ENSURE: truncates by default; strict mode requires equal lengths.
- FAILURE: `CONSTRAINT_VIOLATION` (strict length mismatch); `INVALID_ARGUMENT`.
- Complexity: `Time O(min(n,m))`, `Space O(min(n,m))`.
- Example:
```agentlang
pairs = ZIP(days, totals, mode: strict)
```

### 2.2 core.io

#### core.io.READ
- Type Signature: `READ[T=Bytes](uri: Str, decoder: (Bytes)->T=identity) -> T`
- REQUIRE: valid URI; caller has read capability.
- ENSURE: no source mutation; decoded content returned.
- FAILURE: `NOT_FOUND`; `UNAUTHORIZED`; `IO_ERROR`; `PARSE_ERROR`.
- Complexity: `Time O(n)`, `Space O(n)`.
- Example:
```agentlang
cfg = READ(uri: "file:///etc/agent/config.json", decoder: JSON.DECODE)
```

#### core.io.WRITE
- Type Signature: `WRITE[T](uri: Str, value: T, encoder: (T)->Bytes=identity, mode: ENUM(overwrite,append)=overwrite) -> Hash`
- REQUIRE: valid URI; mode valid; caller has write capability.
- ENSURE: bytes persisted per mode; returns content hash.
- FAILURE: `UNAUTHORIZED`; `IO_ERROR`; `INVALID_ARGUMENT`.
- Complexity: `Time O(n)`, `Space O(n)`.
- Example:
```agentlang
h = WRITE(uri: "s3://bucket/result.bin", value: payload)
```

#### core.io.FETCH
- Type Signature: `FETCH[T=Bytes](uri: Str, timeout: Duration=30s, decoder: (Bytes)->T=identity) -> T`
- REQUIRE: valid remote URI; `timeout > 0`; network read capability.
- ENSURE: timeout policy enforced; decoded payload returned.
- FAILURE: `TIMEOUT`; `UNAUTHORIZED`; `DEPENDENCY_ERROR`; `PARSE_ERROR`.
- Complexity: `Time O(n + latency)`, `Space O(n)`.
- Example:
```agentlang
doc = FETCH(uri: "https://api.example.com/doc", decoder: JSON.DECODE)
```

#### core.io.STREAM
- Type Signature: `STREAM[T](source: Str, decoder: (Bytes)->T, buffer: UInt32=1024) -> Stream[T]`
- REQUIRE: valid stream source; `buffer >= 1`; read capability.
- ENSURE: lazy stream; order preserved; backpressure supported.
- FAILURE: `IO_ERROR`; `TIMEOUT`; `PARSE_ERROR`.
- Complexity: `Time O(n total)`, `Space O(buffer)`.
- Example:
```agentlang
lines = STREAM(source: "file:///var/log/app.log", decoder: UTF8.LINE)
```

### 2.3 core.math

#### core.math.ARITHMETIC
- Type Signature: `ARITHMETIC[N: Numeric](op: ENUM(add,sub,mul,div,mod,pow), a: N, b: N) -> N`
- REQUIRE: op valid; numeric domain valid; divisor non-zero for `div`/`mod`.
- ENSURE: result equals operation semantics.
- FAILURE: `INVALID_ARGUMENT`; `CONSTRAINT_VIOLATION` (zero-divide/overflow).
- Complexity: `Time O(1)`, `Space O(1)`.
- Example:
```agentlang
net = ARITHMETIC(op: sub, a: revenue, b: costs)
```

#### core.math.TRIG
- Type Signature: `TRIG[N: Float](op: ENUM(sin,cos,tan,asin,acos,atan), x: N, unit: ENUM(rad,deg)=rad) -> N`
- REQUIRE: op valid; inverse trig domains satisfied.
- ENSURE: unit-normalized result with IEEE754 precision baseline.
- FAILURE: `INVALID_ARGUMENT`; `CONSTRAINT_VIOLATION`.
- Complexity: `Time O(1)`, `Space O(1)`.
- Example:
```agentlang
angle = TRIG(op: atan, x: imag / real)
```

#### core.math.STATS
- Type Signature: `STATS[N: Numeric](op: ENUM(mean,median,variance,stdev,min,max,quantile), values: List[N], q: Float64=0.5) -> Float64`
- REQUIRE: non-empty values; valid op; `0<=q<=1` for quantile.
- ENSURE: deterministic statistic; non-negative variance/stdev.
- FAILURE: `INVALID_ARGUMENT`; `CONSTRAINT_VIOLATION`.
- Complexity: `Time O(n log n)` worst-case, `Space O(n)`.
- Example:
```agentlang
p95 = STATS(op: quantile, values: latencies, q: 0.95)
```

### 2.4 core.text

#### core.text.PARSE
- Type Signature: `PARSE[T](input: Str, grammar: Grammar[T]) -> T`
- REQUIRE: valid grammar; string input.
- ENSURE: parse tree/value of type `T` or typed parse failure.
- FAILURE: `PARSE_ERROR`; `INVALID_ARGUMENT`.
- Complexity: `Time O(n)` typical (`O(n^3)` worst for ambiguous CFG), `Space O(n)`.
- Example:
```agentlang
ast = PARSE(input: expr_text, grammar: ExprGrammar)
```

#### core.text.FORMAT
- Type Signature: `FORMAT(template: Str, values: Map[Str,Any], strict: Bool=TRUE) -> Str`
- REQUIRE: template syntax valid.
- ENSURE: placeholders substituted; strict mode forbids unresolved slots.
- FAILURE: `VALIDATION_ERROR`; `INVALID_ARGUMENT`.
- Complexity: `Time O(n+k)`, `Space O(n)`.
- Example:
```agentlang
msg = FORMAT(template: "Hello {{name}}", values: {name: user.name})
```

#### core.text.REGEX
- Type Signature: `REGEX(op: ENUM(match,search,replace,split), pattern: Str, input: Str, replacement: Str=NONE) -> RegexResult`
- REQUIRE: valid regex; valid op; replacement provided for replace.
- ENSURE: result corresponds to regex engine semantics for op.
- FAILURE: `PARSE_ERROR`; `INVALID_ARGUMENT`.
- Complexity: `Time engine-dependent`, `Space O(n)`.
- Example:
```agentlang
clean = REGEX(op: replace, pattern: "[0-9]{16}", input: text, replacement: "****")
```

#### core.text.TOKENIZE
- Type Signature: `TOKENIZE(input: Str, strategy: ENUM(whitespace,bpe,wordpiece,sentence)=whitespace) -> TokenizeResult`
- REQUIRE: strategy supported.
- ENSURE: deterministic tokenization for same strategy/version.
- FAILURE: `INVALID_ARGUMENT`.
- Complexity: `Time O(n)`, `Space O(n)`.
- Example:
```agentlang
toks = TOKENIZE(input: prompt_text, strategy: sentence)
```

#### core.text.EMBED
- Type Signature: `EMBED(input: Str|List[Str], model: Str, dims: UInt32=1536) -> Vector[Float32]|List[Vector[Float32]]`
- REQUIRE: model available; `dims > 0`; LLM capability.
- ENSURE: finite vectors with requested dimensionality.
- FAILURE: `UNAUTHORIZED`; `DEPENDENCY_ERROR`; `TIMEOUT`.
- Complexity: `Time O(items*dims)`, `Space O(dims)` per item.
- Example:
```agentlang
v = EMBED(input: article_text, model: "text-embed-3-large", dims: 3072)
```

### 2.5 core.time

#### core.time.NOW
- Type Signature: `NOW(clock: ENUM(utc,monotonic)=utc) -> Timestamp`
- REQUIRE: clock variant supported.
- ENSURE: valid timestamp; monotonic clock non-decreasing.
- FAILURE: `INTERNAL`.
- Complexity: `Time O(1)`, `Space O(1)`.
- Example:
```agentlang
t0 = NOW(clock: utc)
```

#### core.time.DURATION
- Type Signature: `DURATION(start: Timestamp, end: Timestamp, unit: ENUM(ms,s,m,h)=ms) -> Int64`
- REQUIRE: timestamps valid; unit supported.
- ENSURE: signed delta converted to requested unit.
- FAILURE: `INVALID_ARGUMENT`.
- Complexity: `Time O(1)`, `Space O(1)`.
- Example:
```agentlang
elapsed = DURATION(start: t0, end: NOW(), unit: ms)
```

#### core.time.SCHEDULE
- Type Signature: `SCHEDULE(task: ()->T, at: Timestamp|CronExpr, timezone: Str="UTC", policy: RetryPolicy=default) -> TaskId`
- REQUIRE: callable task; valid time expression/timezone; scheduler capability.
- ENSURE: durable registration; audit emission.
- FAILURE: `UNAUTHORIZED`; `VALIDATION_ERROR`; `CONFLICT`.
- Complexity: `Time O(1)` registration, `Space O(1)` metadata.
- Example:
```agentlang
job = SCHEDULE(task: run_etl, at: "0 2 * * *", timezone: "America/New_York")
```

#### core.time.INTERVAL
- Type Signature: `INTERVAL(start: Timestamp, step: Duration, count: UInt32) -> List[Timestamp]`
- REQUIRE: `step > 0`; `count >= 1`.
- ENSURE: arithmetic progression from start with fixed step.
- FAILURE: `INVALID_ARGUMENT`.
- Complexity: `Time O(count)`, `Space O(count)`.
- Example:
```agentlang
ticks = INTERVAL(start: NOW(), step: 5m, count: 12)
```

### 2.6 core.crypto

#### core.crypto.HASH
- Type Signature: `HASH(data: Bytes|Str, algorithm: ENUM(sha256,sha512,blake3)=sha256) -> Hash`
- REQUIRE: supported algorithm; data provided.
- ENSURE: deterministic digest output.
- FAILURE: `INVALID_ARGUMENT`.
- Complexity: `Time O(n)`, `Space O(1)` incremental.
- Example:
```agentlang
digest = HASH(data: payload, algorithm: sha256)
```

#### core.crypto.SIGN
- Type Signature: `SIGN(data: Bytes, private_key: KeyRef, algorithm: ENUM(ed25519,rsa_pss)=ed25519) -> Bytes`
- REQUIRE: key accessible; sign capability.
- ENSURE: signature verifies with corresponding public key.
- FAILURE: `UNAUTHORIZED`; `NOT_FOUND`; `INVALID_ARGUMENT`.
- Complexity: `Time O(n+k)`, `Space O(k)`.
- Example:
```agentlang
sig = SIGN(data: artifact, private_key: KMS("signing-key"))
```

#### core.crypto.VERIFY
- Type Signature: `VERIFY(data: Bytes, signature: Bytes, public_key: KeyRef, algorithm: ENUM(ed25519,rsa_pss)=ed25519) -> Bool`
- REQUIRE: public key accessible; algorithm/key compatible.
- ENSURE: boolean reflects cryptographic verification result.
- FAILURE: `NOT_FOUND`; `INVALID_ARGUMENT`.
- Complexity: `Time O(n+k)`, `Space O(k)`.
- Example:
```agentlang
ok = VERIFY(data: artifact, signature: sig, public_key: KMS("signing-pub"))
```

#### core.crypto.ENCRYPT
- Type Signature: `ENCRYPT(plaintext: Bytes, key: KeyRef, algorithm: ENUM(aes_gcm,chacha20_poly1305)=aes_gcm, aad: Bytes=NONE) -> Bytes`
- REQUIRE: key accessible; encrypt capability.
- ENSURE: ciphertext non-empty; decryptability with same key/params.
- FAILURE: `UNAUTHORIZED`; `NOT_FOUND`; `INVALID_ARGUMENT`.
- Complexity: `Time O(n)`, `Space O(n)`.
- Example:
```agentlang
sealed = ENCRYPT(plaintext: pii, key: KMS("pii-key"), algorithm: aes_gcm)
```

### 2.7 core.json

#### core.json.ENCODE
- Type Signature: `ENCODE[T](value: T, canonical: Bool=FALSE) -> Str`
- REQUIRE: value JSON-serializable.
- ENSURE: decodable JSON; canonical mode sorts keys.
- FAILURE: `TYPE_MISMATCH`.
- Complexity: `Time O(n)`, `Space O(n)`.
- Example:
```agentlang
body = JSON.ENCODE(value: record, canonical: TRUE)
```

#### core.json.DECODE
- Type Signature: `DECODE[T=JsonValue](json: Str, schema: Schema[T]=NONE) -> T`
- REQUIRE: valid JSON text input.
- ENSURE: parsed value returned; optional schema enforced.
- FAILURE: `PARSE_ERROR`; `VALIDATION_ERROR`.
- Complexity: `Time O(n)`, `Space O(n)`.
- Example:
```agentlang
payload = JSON.DECODE(json: request.body, schema: UserSchema)
```

#### core.json.VALIDATE
- Type Signature: `VALIDATE[T](value: T, schema: Schema[T]) -> Bool|FAILURE`
- REQUIRE: schema valid.
- ENSURE: `TRUE` iff all constraints pass; failure includes violation paths.
- FAILURE: `VALIDATION_ERROR`; `INVALID_ARGUMENT`.
- Complexity: `Time O(f)`, `Space O(depth)`.
- Example:
```agentlang
ASSERT JSON.VALIDATE(value: candidate, schema: UserSchema)
```

### 2.8 core.http

#### core.http.GET
- Type Signature: `GET[T=Bytes](url: Str, headers: Map[Str,Str]={}, timeout: Duration=30s, decoder: (Bytes)->T=identity) -> HttpResponse[T]`
- REQUIRE: valid URL; `timeout > 0`; net read capability.
- ENSURE: method is GET; decoded body returned.
- FAILURE: `TIMEOUT`; `DEPENDENCY_ERROR`; `RATE_LIMITED`.
- Complexity: `Time O(n+latency)`, `Space O(n)`.
- Example:
```agentlang
resp = HTTP.GET(url: "https://status.example.com/health", timeout: 3s)
```

#### core.http.POST
- Type Signature: `POST[B,T=Bytes](url: Str, body: B, headers: Map[Str,Str]={}, timeout: Duration=30s, encoder: (B)->Bytes=identity, decoder: (Bytes)->T=identity, idempotency_key: Str=NONE) -> HttpResponse[T]`
- REQUIRE: valid URL; `timeout > 0`; net write capability.
- ENSURE: method is POST; idempotency honored when key set.
- FAILURE: `TIMEOUT`; `DEPENDENCY_ERROR`; `UNAUTHORIZED`.
- Complexity: `Time O(n+latency)`, `Space O(n)`.
- Example:
```agentlang
created = HTTP.POST(url: api_url, body: JSON.ENCODE(new_user), headers: auth)
```

#### core.http.PUT
- Type Signature: `PUT[B,T=Bytes](url: Str, body: B, headers: Map[Str,Str]={}, timeout: Duration=30s, encoder: (B)->Bytes=identity, decoder: (Bytes)->T=identity, idempotency_key: Str=NONE) -> HttpResponse[T]`
- REQUIRE: valid URL; `timeout > 0`; net write capability.
- ENSURE: method is PUT; full-replacement semantics.
- FAILURE: `TIMEOUT`; `DEPENDENCY_ERROR`; `CONFLICT`.
- Complexity: `Time O(n+latency)`, `Space O(n)`.
- Example:
```agentlang
updated = HTTP.PUT(url: user_url, body: JSON.ENCODE(user_v2), idempotency_key: run_id)
```

#### core.http.DELETE
- Type Signature: `DELETE[T=Bytes](url: Str, headers: Map[Str,Str]={}, timeout: Duration=30s, decoder: (Bytes)->T=identity, idempotency_key: Str=NONE) -> HttpResponse[T]`
- REQUIRE: valid URL; `timeout > 0`; net write capability.
- ENSURE: method is DELETE; duplicate-safe behavior with idempotency key.
- FAILURE: `TIMEOUT`; `DEPENDENCY_ERROR`; `UNAUTHORIZED`.
- Complexity: `Time O(n+latency)`, `Space O(n)`.
- Example:
```agentlang
deleted = HTTP.DELETE(url: stale_url, headers: auth)
```

## 3. Agent Modules

### 3.1 agent.llm

#### agent.llm.GENERATE
- Type Signature: `GENERATE[T=Str](prompt: Str, model: Str, response_schema: Schema[T]=NONE, temperature: Float32=0.2, max_tokens: UInt32=512) -> Probable[T]`
- REQUIRE: non-empty prompt; supported model; bounded temperature; LLM capability.
- ENSURE: confidence in `[0,1]`; schema-valid output when schema provided.
- FAILURE: `RATE_LIMITED`; `TIMEOUT`; `DEPENDENCY_ERROR`; `VALIDATION_ERROR`.
- Complexity: `Time O(tokens_in+tokens_out+latency)`, `Space O(tokens_out)`.
- Example:
```agentlang
summary = LLM.GENERATE(prompt: "Summarize incident in 3 bullets", model: "claude-sonnet-4-20250514")
```

#### agent.llm.CLASSIFY
- Type Signature: `CLASSIFY[L](input: Str, labels: List[L], model: Str, min_confidence: Confidence=~0.0) -> Probable[L]`
- REQUIRE: non-empty input; at least 2 labels; supported model.
- ENSURE: output label in provided set; confidence threshold respected.
- FAILURE: `CONSTRAINT_VIOLATION`; `RATE_LIMITED`; `TIMEOUT`.
- Complexity: `Time O(tokens+labels+latency)`, `Space O(labels)`.
- Example:
```agentlang
intent = LLM.CLASSIFY(input: msg, labels: [question, command, abuse], model: "claude-sonnet-4-20250514", min_confidence: ~0.75)
```

#### agent.llm.EXTRACT
- Type Signature: `EXTRACT[T](input: Str, schema: Schema[T], model: Str, strict: Bool=TRUE) -> Probable[T]`
- REQUIRE: non-empty input; valid schema; supported model.
- ENSURE: output conforms to schema; strict mode forbids extra fields.
- FAILURE: `VALIDATION_ERROR`; `TIMEOUT`; `DEPENDENCY_ERROR`.
- Complexity: `Time O(tokens+latency)`, `Space O(size(T))`.
- Example:
```agentlang
invoice = LLM.EXTRACT(input: invoice_text, schema: InvoiceSchema, model: "claude-sonnet-4-20250514", strict: TRUE)
```

### 3.2 agent.memory

#### agent.memory.REMEMBER
- Type Signature: `REMEMBER[K:Str,V](key: K, value: V, ttl: Duration=NONE, namespace: Str="default") -> Hash`
- REQUIRE: non-empty key/namespace; serializable value; memory write capability.
- ENSURE: value recallable until expiry; hash returned; audit emitted.
- FAILURE: `UNAUTHORIZED`; `CONSTRAINT_VIOLATION`; `INTERNAL`.
- Complexity: `Time O(1)` expected, `Space O(size(value))`.
- Example:
```agentlang
memo = MEMORY.REMEMBER(key: "session:42:intent", value: intent, ttl: 2h)
```

#### agent.memory.RECALL
- Type Signature: `RECALL[K:Str,V](key: K, namespace: Str="default", default: V=NONE) -> V|NONE`
- REQUIRE: non-empty key/namespace; memory read capability.
- ENSURE: returns stored value if present and fresh, else default.
- FAILURE: `UNAUTHORIZED`; `INTERNAL`.
- Complexity: `Time O(1)` expected, `Space O(1)`.
- Example:
```agentlang
last = MEMORY.RECALL(key: "session:42:intent", default: NONE)
```

#### agent.memory.FORGET
- Type Signature: `FORGET(key: Str, namespace: Str="default") -> Bool`
- REQUIRE: non-empty key; memory write capability.
- ENSURE: key absent after success; idempotent deletion semantics.
- FAILURE: `UNAUTHORIZED`; `INTERNAL`.
- Complexity: `Time O(1)` expected, `Space O(1)`.
- Example:
```agentlang
ok = MEMORY.FORGET(key: "session:42:intent")
```

### 3.3 agent.tools

#### agent.tools.REGISTER_TOOL
- Type Signature: `REGISTER_TOOL(name: Str, signature: ToolSignature, handler: ToolHandler, policy: ToolPolicy=default) -> ToolId`
- REQUIRE: valid unique name; valid signature; callable handler; register capability.
- ENSURE: registry entry exists; runtime validation enabled.
- FAILURE: `CONFLICT`; `UNAUTHORIZED`; `VALIDATION_ERROR`.
- Complexity: `Time O(1)`, `Space O(1)` metadata.
- Example:
```agentlang
tid = TOOLS.REGISTER_TOOL(name: "weather.lookup", signature: WeatherSig, handler: weather_lookup)
```

#### agent.tools.INVOKE
- Type Signature: `INVOKE[I,O](tool: Str, input: I, timeout: Duration=30s, retry: UInt8=0) -> O`
- REQUIRE: tool registered; input matches declared schema; invoke capability.
- ENSURE: output matches declared schema; retries bounded by policy.
- FAILURE: `NOT_FOUND`; `VALIDATION_ERROR`; `TIMEOUT`; `DEPENDENCY_ERROR`.
- Complexity: `Time O(tool_runtime + retries)`, `Space O(input+output)`.
- Example:
```agentlang
wx = TOOLS.INVOKE(tool: "weather.lookup", input: {city: "Boston"}, timeout: 5s, retry: 1)
```

### 3.4 agent.planning

#### agent.planning.PLAN
- Type Signature: `PLAN(goal: Str, context: Map[Str,Any]={}, constraints: List[Constraint]=[]) -> Plan`
- REQUIRE: non-empty goal; hard constraints satisfiable or explicitly relaxed.
- ENSURE: non-empty step graph; hard constraints represented in plan metadata.
- FAILURE: `CONSTRAINT_VIOLATION`; `INVALID_ARGUMENT`.
- Complexity: `Time O(s*c)` heuristic, `Space O(s)`.
- Example:
```agentlang
plan = PLANNING.PLAN(goal: "Reduce API p95 latency below 200ms", constraints: [no_downtime])
```

#### agent.planning.DECOMPOSE
- Type Signature: `DECOMPOSE(task: Task, strategy: ENUM(depth_first,breadth_first,cost_first)=cost_first, max_depth: UInt8=4) -> List[Task]`
- REQUIRE: non-terminal task; valid strategy; `max_depth>=1`.
- ENSURE: subtasks preserve parent goal semantics; depth bounded.
- FAILURE: `CONSTRAINT_VIOLATION`; `INVALID_ARGUMENT`.
- Complexity: `Time O(b^d)` worst-case, `Space O(b^d)`.
- Example:
```agentlang
subs = PLANNING.DECOMPOSE(task: plan.root, strategy: cost_first, max_depth: 3)
```

### 3.5 agent.reflection

#### agent.reflection.EVALUATE
- Type Signature: `EVALUATE[T](artifact: T, rubric: Rubric, evidence: List[Any]=[]) -> Evaluation`
- REQUIRE: rubric has criteria; artifact evaluable; reflect capability.
- ENSURE: score in `[0,100]`; criterion coverage complete.
- FAILURE: `INVALID_ARGUMENT`; `DEPENDENCY_ERROR`.
- Complexity: `Time O(c+e)`, `Space O(c+e)`.
- Example:
```agentlang
ev = REFLECTION.EVALUATE(artifact: patch, rubric: CodeSafetyRubric, evidence: test_logs)
```

#### agent.reflection.CRITIQUE
- Type Signature: `CRITIQUE[T](artifact: T, mode: ENUM(risks,correctness,style,all)=all) -> List[CritiqueItem]`
- REQUIRE: artifact analyzable; mode valid; reflect capability.
- ENSURE: findings ordered by severity; each finding includes actionable recommendation.
- FAILURE: `INVALID_ARGUMENT`; `DEPENDENCY_ERROR`.
- Complexity: `Time O(n)`, `Space O(k)`.
- Example:
```agentlang
issues = REFLECTION.CRITIQUE(artifact: plan, mode: risks)
```

## 4. Integration Modules

### 4.1 db.sql

#### db.sql.QUERY
- Type Signature: `QUERY[R](conn: DbConn, sql: Str, params: List[Any]=[], mapper: (Row)->R=identity) -> List[R]`
- REQUIRE: open connection; parameterized/safe SQL; DB read capability.
- ENSURE: mapped rows returned without write side effects.
- FAILURE: `UNAUTHORIZED`; `VALIDATION_ERROR`; `DEPENDENCY_ERROR`; `TIMEOUT`.
- Complexity: `Time O(query_plan + rows)`, `Space O(rows)`.
- Example:
```agentlang
rows = SQL.QUERY(conn: db, sql: "SELECT id,email FROM users WHERE active = ?", params: [TRUE])
```

#### db.sql.INSERT
- Type Signature: `INSERT[T](conn: DbConn, table: Str, record: T, returning: List[Str]=[]) -> DbWriteResult`
- REQUIRE: table exists; record conforms to schema; DB write capability.
- ENSURE: atomic single-row insert; requested returning fields provided.
- FAILURE: `UNAUTHORIZED`; `CONSTRAINT_VIOLATION`; `DEPENDENCY_ERROR`.
- Complexity: `Time O(1 + index_updates)`, `Space O(1)`.
- Example:
```agentlang
ins = SQL.INSERT(conn: db, table: "audit_log", record: event, returning: ["id"])
```

#### db.sql.UPDATE
- Type Signature: `UPDATE(conn: DbConn, table: Str, set: Map[Str,Any], where: SqlPredicate, limit: UInt32=0) -> DbWriteResult`
- REQUIRE: explicit predicate; non-empty updates; DB write capability.
- ENSURE: only matching rows modified; limit respected when non-zero.
- FAILURE: `UNAUTHORIZED`; `VALIDATION_ERROR`; `CONSTRAINT_VIOLATION`; `DEPENDENCY_ERROR`.
- Complexity: `Time O(scan/index + affected_rows)`, `Space O(1)`.
- Example:
```agentlang
upd = SQL.UPDATE(conn: db, table: "jobs", set: {status: "running"}, where: EQ("id", job_id), limit: 1)
```

### 4.2 db.vector

#### db.vector.UPSERT
- Type Signature: `UPSERT[I:Str](conn: VectorConn, id: I, vector: Vector[Float32], metadata: Map[Str,Any]={}, namespace: Str="default") -> Bool`
- REQUIRE: open connection; id non-empty; dimension match; DB write capability.
- ENSURE: record created or replaced atomically in namespace.
- FAILURE: `UNAUTHORIZED`; `VALIDATION_ERROR`; `DEPENDENCY_ERROR`.
- Complexity: `Time O(d + log n)` typical, `Space O(d)`.
- Example:
```agentlang
ok = VECTOR.UPSERT(conn: vdb, id: doc_id, vector: doc_vec, metadata: {source: "kb"})
```

#### db.vector.SIMILARITY_SEARCH
- Type Signature: `SIMILARITY_SEARCH(conn: VectorConn, query: Vector[Float32], top_k: UInt32=10, filter: Map[Str,Any]={}, namespace: Str="default") -> List[VectorMatch]`
- REQUIRE: query dimension matches index; `top_k>=1`; DB read capability.
- ENSURE: at most `top_k` matches; descending similarity scores; filter respected.
- FAILURE: `UNAUTHORIZED`; `VALIDATION_ERROR`; `DEPENDENCY_ERROR`; `TIMEOUT`.
- Complexity: `Time O(search_cost + top_k log top_k)`, `Space O(top_k)`.
- Example:
```agentlang
hits = VECTOR.SIMILARITY_SEARCH(conn: vdb, query: q_vec, top_k: 5, filter: {tenant: tenant_id})
```

### 4.3 api.rest

#### api.rest.ENDPOINT
- Type Signature: `ENDPOINT(method: ENUM(GET,POST,PUT,DELETE,PATCH), path: Str, request_schema: Schema[Any]=NONE, response_schema: Schema[Any]=NONE, handler: HttpHandler) -> RouteId`
- REQUIRE: path begins with `/`; method valid; callable handler; API define capability.
- ENSURE: unique route registered; request/response validation enabled when schemas present.
- FAILURE: `CONFLICT`; `VALIDATION_ERROR`; `UNAUTHORIZED`.
- Complexity: `Time O(1)` registration, `Space O(1)`.
- Example:
```agentlang
rid = REST.ENDPOINT(method: POST, path: "/v1/users", request_schema: UserCreateSchema, response_schema: UserSchema, handler: create_user)
```

#### api.rest.MIDDLEWARE
- Type Signature: `MIDDLEWARE(scope: Str="*", before: MiddlewareFn=NONE, after: MiddlewareFn=NONE, on_error: ErrorMiddlewareFn=NONE, order: Int32=0) -> MiddlewareId`
- REQUIRE: valid scope; at least one hook provided; API define capability.
- ENSURE: deterministic middleware chain order.
- FAILURE: `VALIDATION_ERROR`; `CONFLICT`; `UNAUTHORIZED`.
- Complexity: `Time O(1)` registration; per-request overhead `O(m)`.
- Example:
```agentlang
mw = REST.MIDDLEWARE(scope: "/v1/*", before: require_auth, on_error: auth_err, order: 10)
```

### 4.4 queue.pubsub

#### queue.pubsub.PUBLISH
- Type Signature: `PUBLISH[T](topic: Str, message: T, key: Str=NONE, headers: Map[Str,Str]={}, delay: Duration=0ms) -> MessageId`
- REQUIRE: non-empty topic; serializable message; `delay>=0`; publish capability.
- ENSURE: message durably enqueued; per-key ordering preserved where supported.
- FAILURE: `UNAUTHORIZED`; `DEPENDENCY_ERROR`; `CONSTRAINT_VIOLATION`.
- Complexity: `Time O(1)`, `Space O(size(message))`.
- Example:
```agentlang
mid = PUBSUB.PUBLISH(topic: "orders.created", message: event, key: event.id)
```

#### queue.pubsub.SUBSCRIBE
- Type Signature: `SUBSCRIBE[T](topic: Str, group: Str, handler: (Message[T])->Ack, offset: ENUM(earliest,latest)=latest, max_inflight: UInt32=64) -> SubscriptionId`
- REQUIRE: non-empty topic/group; callable handler; `max_inflight>=1`; subscribe capability.
- ENSURE: active consumer registration; at-least-once delivery with backpressure limits.
- FAILURE: `UNAUTHORIZED`; `DEPENDENCY_ERROR`; `CONFLICT`.
- Complexity: `Time O(messages_processed)`, `Space O(max_inflight)`.
- Example:
```agentlang
sid = PUBSUB.SUBSCRIBE(topic: "orders.created", group: "billing-workers", handler: handle_order, offset: latest, max_inflight: 128)
```

## 5. Standard Operation Templates (Normative)

### 5.1 Operation Declaration Template

```agentlang
OPERATION module.operation =>
    INPUT ...
    OUTPUT ...
    REQUIRE ...
    ENSURE ...
    BODY {
        ...
        EMIT ...
    }
```

### 5.2 Failure Handling Template

```agentlang
result = HTTP.GET(url: endpoint)
MATCH result =>
    WHEN FAILURE(TIMEOUT, msg, details) -> RETRY(3, backoff: exponential)
    WHEN FAILURE(RATE_LIMITED, msg, details) -> RETRY(2, backoff: linear)
    WHEN FAILURE(_, msg, details) -> ESCALATE(msg)
```

### 5.3 Capability Gating Template

```agentlang
OPERATION guarded_write =>
    REQUIRE caller HAS CAPABILITY DB_WRITE
    ENSURE audit_event_emitted
    BODY {
        SQL.INSERT(conn: db, table: "audit_log", record: event)
        EMIT TRUE
    }
```

## 6. Coverage Index

### 6.1 Core Modules Covered

- `core.data`: `FILTER`, `MAP`, `REDUCE`, `SORT`, `GROUP`, `TAKE`, `SKIP`, `FLATTEN`, `DISTINCT`, `ZIP`
- `core.io`: `READ`, `WRITE`, `FETCH`, `STREAM`
- `core.math`: `ARITHMETIC`, `TRIG`, `STATS`
- `core.text`: `PARSE`, `FORMAT`, `REGEX`, `TOKENIZE`, `EMBED`
- `core.time`: `NOW`, `DURATION`, `SCHEDULE`, `INTERVAL`
- `core.crypto`: `HASH`, `SIGN`, `VERIFY`, `ENCRYPT`
- `core.json`: `ENCODE`, `DECODE`, `VALIDATE`
- `core.http`: `GET`, `POST`, `PUT`, `DELETE`

### 6.2 Agent Modules Covered

- `agent.llm`: `GENERATE`, `CLASSIFY`, `EXTRACT`
- `agent.memory`: `REMEMBER`, `RECALL`, `FORGET`
- `agent.tools`: `REGISTER_TOOL`, `INVOKE`
- `agent.planning`: `PLAN`, `DECOMPOSE`
- `agent.reflection`: `EVALUATE`, `CRITIQUE`

### 6.3 Integration Modules Covered

- `db.sql`: `QUERY`, `INSERT`, `UPDATE`
- `db.vector`: `UPSERT`, `SIMILARITY_SEARCH`
- `api.rest`: `ENDPOINT`, `MIDDLEWARE`
- `queue.pubsub`: `PUBLISH`, `SUBSCRIBE`

### 6.4 Total Operations

- `58` operations specified with type signatures, `REQUIRE`, `ENSURE`, `FAILURE`, complexity, and example syntax.

## 7. Additional Normative Notes

### 7.1 Determinism Classes

- `Deterministic`: same inputs and environment produce identical outputs (`core.data`, most `core.math`, `core.json`).
- `Conditionally Deterministic`: deterministic under explicit external state assumptions (`db.sql`, `db.vector`, `queue.pubsub`).
- `Probabilistic`: output distribution depends on stochastic inference (`agent.llm`, some `agent.reflection`).

### 7.2 Idempotency Guidance

- Operations naturally idempotent by key: `agent.memory.REMEMBER` (same key/version), `db.vector.UPSERT`, `api.rest.ENDPOINT` (registration conflict guarded).
- Operations idempotent when keyed: `core.http.POST`, `core.http.PUT`, `core.http.DELETE`, `queue.pubsub.PUBLISH`.
- Non-idempotent by default: `db.sql.INSERT`, `core.io.WRITE` in append mode.

### 7.3 Recommended Failure Mapping

- `TIMEOUT` and `DEPENDENCY_ERROR` -> bounded `RETRY` with backoff.
- `RATE_LIMITED` -> retry with provider-specific cool-down and jitter.
- `VALIDATION_ERROR`, `TYPE_MISMATCH`, `INVALID_ARGUMENT` -> fail-fast without retry.
- `UNAUTHORIZED` -> escalate and request capability or credential refresh.

### 7.4 Runtime Audit Fields

For side-effecting operations, runtime SHOULD record:

- `operation_name`
- `timestamp_utc`
- `agent_id`
- `input_hash`
- `output_hash_or_failure`
- `latency_ms`
- `retry_count`
- `capability_check_result`

### 7.5 Versioning Rule

Breaking contract changes MUST increment major version of the owning module namespace.
