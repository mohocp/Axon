# AgentLang MVP Profile v0.1

Status: Normative profile for the MVP implementation target.

## 1. Scope

This profile defines the only features, operators, modules, and semantics that are in scope for AgentLang MVP v0.1. Anything not explicitly listed as included is out of scope and must fail with a compile-time `NOT_IMPLEMENTED` diagnostic.

## 2. Language Feature Freeze

### Included

| Area | Included in MVP v0.1 |
|---|---|
| Core declarations | `SCHEMA`, `TYPE`, `AGENT`, `OPERATION`, `PIPELINE` |
| Dataflow execution | Pipeline chaining via `->` and `|>` |
| State model | `STORE`, `MUTABLE`, assignment (`name = expr`) |
| Control flow | `MATCH`/`WHEN`/`OTHERWISE`, bounded `LOOP`, `HALT`, `RETRY`, `ESCALATE`, `EMIT` |
| Result handling | `SUCCESS(...)` and canonical `FAILURE(ErrorCode, message: Str, details: FailureDetails)` |
| Checkpointing | `CHECKPOINT`, `RESUME` |
| Concurrency | `FORK` with `JOIN strategy: ALL_COMPLETE` only |
| Multi-agent | `DELEGATE ... TO ...` (callee runs with callee capabilities) |
| Verification clauses | `REQUIRE`, `ENSURE`, `INVARIANT`, `ASSERT` |

### Excluded

| Area | Explicitly excluded from MVP v0.1 |
|---|---|
| Reactive/channel runtime | `OBSERVE`, `BROADCAST`, `EMIT TO channel => ...` |
| Sync operator | `<=>` |
| Join variants | `BEST_EFFORT`, `PARTIAL(min=k)` |
| Self-modification flow | `MUTATE OPERATION`, runtime proof protocol orchestration |
| Advanced streaming semantics | Full `STREAM`/reactive reduction semantics |
| Dynamic agent discovery | language-level `DISCOVER` |

## 3. Operator Freeze

### Included operators

`->`, `|>`, `=>`, `:`, `::`, `?`, `@`, `#`, `..`

### Excluded operators

`<=>`, `!`

Notes:
- `|>` is canonicalized to the same stage-application semantics as `->` in MVP v0.1.
- `!` force/override semantics are deferred because policy interaction is under-specified.

## 4. Canonical Result and FAILURE Shape

`FAILURE` arity is fixed to 3 fields for MVP v0.1.

```agentlang
TYPE FailureDetails = Map[Str, JsonValue] | NONE
TYPE Result[T] = SUCCESS(T) | FAILURE(ErrorCode, message: Str, details: FailureDetails)
```

Field meaning:
- `code`: canonical `ErrorCode`
- `message`: human-readable summary
- `details`: machine-readable structured context (`Map[Str, JsonValue]`) or `NONE`

All operation contracts and pattern matches must use the 3-field form. Two-field forms are non-conformant in this profile.

## 5. Canonical Capability Registry

Canonical capability identifiers for MVP v0.1:

- `DB_READ`
- `DB_WRITE`
- `FILE_READ`
- `FILE_WRITE`
- `API_CALL`
- `API_DEFINE`
- `QUEUE_PUBLISH`
- `QUEUE_SUBSCRIBE`
- `LLM_INFER`
- `MEMORY_READ`
- `MEMORY_WRITE`
- `TOOL_REGISTER`
- `TOOL_INVOKE`
- `REFLECT`
- `SCHEDULER`
- `DELEGATE`
- `CRYPTO_SIGN`
- `CRYPTO_ENCRYPT`
- `NETWORK_RAW`
- `AGENT_SPAWN`
- `SELF_MODIFY`
- `ESCALATE_HUMAN`

### Deprecated aliases accepted by parser/type-checker

| Deprecated alias in specs/contracts | Canonical capability |
|---|---|
| `read capability` | `FILE_READ` |
| `write capability` | `FILE_WRITE` |
| `network read capability` | `API_CALL` |
| `network write capability` | `API_CALL` |
| `net read capability` | `API_CALL` |
| `net write capability` | `API_CALL` |
| `LLM capability` | `LLM_INFER` |
| `memory read capability` | `MEMORY_READ` |
| `memory write capability` | `MEMORY_WRITE` |
| `register capability` | `TOOL_REGISTER` |
| `invoke capability` | `TOOL_INVOKE` |
| `reflect capability` | `REFLECT` |
| `scheduler capability` | `SCHEDULER` |
| `API define capability` | `API_DEFINE` |
| `publish capability` | `QUEUE_PUBLISH` |
| `subscribe capability` | `QUEUE_SUBSCRIBE` |
| `sign capability` | `CRYPTO_SIGN` |
| `encrypt capability` | `CRYPTO_ENCRYPT` |

## 6. Standard Library Freeze

### Included modules and operations (MVP v0.1)

| Module | Included operations |
|---|---|
| `core.data` | `FILTER`, `MAP`, `REDUCE`, `SORT`, `GROUP`, `TAKE`, `SKIP` |
| `core.io` | `READ`, `WRITE`, `FETCH` |
| `core.text` | `PARSE`, `FORMAT`, `REGEX`, `TOKENIZE` |
| `core.http` | `GET`, `POST` |
| `agent.llm` | `GENERATE`, `CLASSIFY`, `EXTRACT` |
| `agent.memory` | `REMEMBER`, `RECALL`, `FORGET` |

### Excluded modules and operations (MVP v0.1)

| Namespace | Excluded in MVP v0.1 |
|---|---|
| Full modules | `db.graph`, `api.grpc` |
| `queue.pubsub` | all operations (`PUBLISH`, `SUBSCRIBE`, `ACK`, `NACK`, `REPLAY`) |
| `db.sql` | all operations (`QUERY`, `INSERT`, `UPDATE`, `DELETE`, `MIGRATE`) |
| `db.vector` | all operations (`UPSERT`, `SIMILARITY_SEARCH`, `CLUSTER`) |
| `api.rest` | all operations (`ENDPOINT`, `MIDDLEWARE`, `VALIDATE`, `RESPOND`) |
| `core.http` | `PUT`, `DELETE` |
| `core.io` | `STREAM` |
| `core.math` | all operations |
| `core.time` | all operations |
| `core.crypto` | all operations |
| `core.json` | all operations |
| `agent.tools` | all operations |
| `agent.planning` | all operations |
| `agent.reflection` | all operations |
| Additional promised ops | `SUMMARIZE`, `SEARCH`, `DISCOVER`, `PRIORITIZE`, `REPLAN`, `IMPROVE`, `LEARN`, `PIVOT`, `WINDOW`, `ACCUMULATE` |

## 7. Foundational Type Definitions Required by Included Modules

```agentlang
TYPE JsonValue =
    Str | Float64 | Bool | NONE |
    List[JsonValue] | Map[Str, JsonValue]

TYPE HttpStatus = UInt16 :: range(100..599)
TYPE HttpHeaders = Map[Str, Str]
TYPE HttpResponse[T] = {
    status: HttpStatus,
    headers: HttpHeaders,
    body: T,
    request_id: Str | NONE,
    duration_ms: UInt64,
    received_at: Timestamp
}

TYPE RegexGroup = Str | NONE
TYPE RegexMatch = {
    start: UInt32,
    end: UInt32,
    text: Str,
    groups: List[RegexGroup]
}

TYPE RegexResult = Bool | Str | List[Str] | List[RegexMatch] | NONE

TYPE Token = Str | UInt32
TYPE TokenSequence = List[Token]
TYPE TokenizeResult = TokenSequence
```

MVP note:
- `core.text.REGEX` remains a single operation in MVP but must return `RegexResult` (not `Any`).
- `core.text.TOKENIZE` must return `TokenizeResult` (not `List[Any]`).

## 8. Delegation Capability Boundary

MVP v0.1 rule:
- `DELEGATE` executes under callee capabilities, not caller capabilities.
- Caller must hold `DELEGATE` capability.
- No implicit capability inheritance or intersection override is allowed in MVP.

## 9. Conformance Guardrails

- Parser/type-checker/runtime must reject excluded syntax/features with deterministic diagnostics.
- Deferred modules/operations must fail at compile time with `NOT_IMPLEMENTED` and profile tag `mvp-0.1`.
- Any use of non-canonical capability names should emit a deprecation warning and normalize to canonical capability IDs.

## 10. SMT Unknown Boundary (Normative)

Compiler/solver interface result space:
- `Valid`
- `Invalid(counterexample)`
- `Unknown(reason)`

MVP v0.1 rule for `Unknown`:
- The compiler must fail-open at compile time by auto-inserting a runtime `ASSERT` for the unresolved verification condition.
- Runtime must fail-closed: if an auto-inserted `ASSERT` evaluates to false, execution must stop with `FAILURE(ASSERTION_FAILED, message: Str, details: FailureDetails)`.
- Inserted `ASSERT` checks must be recorded in audit output with VC id and solver reason.
