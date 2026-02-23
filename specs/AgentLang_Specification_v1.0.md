# AGENTLANG

## Agent-Native Programming Language

### Full Language Specification v1.0

*A programming language designed for AI agent cognition, optimized for semantic density, formal verification, and multi-agent coordination.*

**Draft Specification — February 2026**

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Lexical Structure](#2-lexical-structure)
3. [Type System](#3-type-system)
4. [Execution Model](#4-execution-model)
5. [Memory Model](#5-memory-model)
6. [Agent Model](#6-agent-model)
7. [Verification System](#7-verification-system)
8. [Standard Library](#8-standard-library)
9. [State Management and Checkpointing](#9-state-management-and-checkpointing)
10. [Concurrency Model](#10-concurrency-model)
11. [Interoperability](#11-interoperability)
12. [Tooling and Ecosystem](#12-tooling-and-ecosystem)
13. [Security Model](#13-security-model)
14. [Complete Example Program](#14-complete-example-program)
15. [Formal Grammar (EBNF)](#15-formal-grammar-ebnf)
16. [Comparison with Existing Languages](#16-comparison-with-existing-languages)
17. [Future Directions](#17-future-directions)

---

## 1. Introduction

### 1.1 Purpose

AgentLang is a programming language designed from first principles for AI agent cognition. Unlike traditional programming languages that optimize for human readability, developer ergonomics, and human mental models, AgentLang optimizes for semantic density, computational efficiency, formal verifiability, and native multi-agent coordination.

The language acknowledges that AI agents, particularly those built on large language models (LLMs), process information fundamentally differently from human developers. They reason through token sequences, operate probabilistically, require explicit state serialization for continuity, and must coordinate with other autonomous agents in parallel execution environments.

### 1.2 Design Philosophy

AgentLang is built on five core design principles:

- **Semantic Density:** Every token carries maximum meaning with zero syntactic decoration. The language eliminates boilerplate, ceremony, and any construct that exists solely for human convenience.

- **Verification by Construction:** Programs carry their own correctness proofs. Every operation includes pre-conditions, post-conditions, and invariants that are mechanically verified before execution.

- **Parallel by Default:** Sequential execution is the exception, not the rule. The dataflow execution model automatically parallelizes independent operations without explicit concurrency primitives.

- **Probabilistic Awareness:** The type system natively represents uncertainty, confidence bounds, and degradation policies, reflecting the inherent probabilistic nature of AI agent operations.

- **Agent-Native Coordination:** Delegation, capability scoping, trust levels, shared state, and conflict resolution are language-level primitives, not library abstractions.

### 1.3 Non-Goals

AgentLang explicitly does not optimize for:

- **Human readability as a primary concern:** The language happens to be semi-readable by humans because semantic richness serves both agents and humans, but human comprehension is a beneficial side effect, not a design constraint.

- **Backward compatibility:** AgentLang does not inherit conventions from existing languages. Familiar syntax patterns from Python, JavaScript, or C are only adopted when they demonstrably serve agent cognition.

- **Manual memory management:** Agents should not reason about allocation, deallocation, or garbage collection. The runtime handles all memory concerns through content-addressable storage.

### 1.4 Target Audience

The primary consumers of AgentLang are AI agents: autonomous software entities that generate, execute, modify, and reason about code. Human developers interact with AgentLang primarily through agent interfaces, debugging tools, and verification audit trails rather than direct code authorship.

---

## 2. Lexical Structure

### 2.1 Character Set and Encoding

AgentLang source is encoded in UTF-8. All keywords are ASCII. String literals and identifiers may contain any Unicode code point. The language is case-sensitive.

### 2.2 Keywords

AgentLang uses semantically rich, single-word keywords designed to maximize meaning per token. Keywords are organized by domain:

#### 2.2.1 Data Operations

| Keyword | Semantics | Python Equivalent |
|---|---|---|
| `FILTER` | Select elements matching predicate | `filter()` / list comprehension |
| `MAP` | Transform each element | `map()` / list comprehension |
| `REDUCE` | Aggregate elements to single value | `functools.reduce()` |
| `SORT` | Order elements by key | `sorted()` |
| `GROUP` | Partition elements by key | `itertools.groupby()` |
| `TAKE` | Select first N elements | `slice[:n]` |
| `SKIP` | Bypass first N elements | `slice[n:]` |
| `FLATTEN` | Reduce nesting depth by one level | `itertools.chain.from_iterable()` |
| `DISTINCT` | Remove duplicate elements | `set()` |
| `ZIP` | Combine parallel sequences | `zip()` |
| `PIVOT` | Reshape tabular data | `pandas pivot` |
| `WINDOW` | Sliding window over sequence | Custom implementation |
| `ACCUMULATE` | Running aggregation with state | `itertools.accumulate()` |

#### 2.2.2 Control Flow

| Keyword | Semantics | Description |
|---|---|---|
| `WHEN` | Conditional branch | Replaces if/else with pattern matching |
| `MATCH` | Pattern matching dispatch | Multi-way conditional with exhaustiveness check |
| `LOOP` | Bounded iteration | Always requires explicit bound to prevent infinite loops |
| `EMIT` | Produce output value | Replaces return; supports streaming |
| `HALT` | Terminate execution path | With mandatory reason code |
| `RETRY` | Re-attempt with policy | Built-in exponential backoff and limit |
| `ESCALATE` | Delegate to higher authority | Transfer control to supervisor agent or human |

#### 2.2.3 Agent Coordination

| Keyword | Semantics | Description |
|---|---|---|
| `AGENT` | Declare agent identity | Defines capabilities, trust level, and scope |
| `DELEGATE` | Assign task to another agent | With timeout, fallback, and isolation policy |
| `BROADCAST` | Send to all agents in scope | Pub/sub style multi-agent communication |
| `ACQUIRE` | Request resource lock | With timeout and deadlock prevention |
| `RELEASE` | Relinquish resource lock | Automatic on scope exit |
| `CHECKPOINT` | Serialize execution state | For pause/resume/migration |
| `RESUME` | Restore from checkpoint | Validate state integrity before resuming |
| `OBSERVE` | Subscribe to state changes | Reactive event-driven coordination |

#### 2.2.4 Verification

| Keyword | Semantics | Description |
|---|---|---|
| `REQUIRE` | Pre-condition assertion | Must hold before operation executes |
| `ENSURE` | Post-condition assertion | Must hold after operation completes |
| `INVARIANT` | Persistent constraint | Must hold throughout scope lifetime |
| `PROVE` | Request formal proof | Triggers verification engine |
| `ASSUME` | Declare unverified premise | Explicit marking of trusted-but-unproven facts |
| `ASSERT` | Runtime check | Fails execution immediately if violated |

#### 2.2.5 Data Declaration

| Keyword | Semantics | Description |
|---|---|---|
| `STORE` | Persistent named data reference | Content-addressable, immutable by default |
| `MUTABLE` | Explicitly mutable binding | Requires justification annotation |
| `SCHEMA` | Data structure definition | With validation rules and constraints |
| `CONST` | Compile-time constant | Inlined at all usage sites |
| `STREAM` | Unbounded data sequence | Lazy evaluation with backpressure |
| `CACHE` | Memoized computation result | With TTL and invalidation policy |

### 2.3 Operators

AgentLang uses a minimal operator set. All operators are symbolic (not keyword-based) to maintain token efficiency while preserving semantic clarity in keywords.

| Operator | Name | Example |
|---|---|---|
| `->` | Pipeline / Transform | `data -> FILTER active -> SORT revenue` |
| `\|>` | Pipe with context | `data \|> TRANSFORM(ctx)` |
| `=>` | Produces / Maps to | `SCHEMA User => {name: Str, age: Int}` |
| `:` | Type annotation | `revenue: Float64` |
| `::` | Constraint annotation | `latency :: <50ms` |
| `?` | Confidence query | `result? >= 0.85` |
| `!` | Force / Override | `EXECUTE! (bypass soft constraints)` |
| `@` | Decorator / Metadata | `@cached @timeout(5s)` |
| `#` | Tag / Label | `#priority_high #team_alpha` |
| `..` | Range | `1..100` |
| `<=>` | Bidirectional sync | `local_state <=> remote_state` |

### 2.4 Literals

AgentLang supports the following literal types:

- **Integers:** `42`, `-7`, `0xFF`, `0b1010`
- **Floats:** `3.14`, `-0.5`, `1.0e-10`
- **Strings:** `"hello"`, `"multi-line strings with \n"`
- **Booleans:** `TRUE`, `FALSE`
- **Null:** `NONE` (explicit absence, must be handled)
- **Duration:** `5s`, `100ms`, `2m`, `1h`
- **Size:** `256KB`, `1MB`, `4GB`
- **Confidence:** `~0.95` (probabilistic value between 0 and 1)
- **Hash:** `SHA256:a3f8...` (content-addressable reference)

---

## 3. Type System

### 3.1 Overview

AgentLang employs a probabilistic dependent type system. Every value carries not only its data type but also a confidence score, provenance chain, and degradation policy. The type system is statically checked where possible and dynamically verified at runtime for probabilistic components.

### 3.2 Primitive Types

| Type | Description | Size |
|---|---|---|
| `Int8`, `Int16`, `Int32`, `Int64` | Signed integers | 1-8 bytes |
| `UInt8`, `UInt16`, `UInt32`, `UInt64` | Unsigned integers | 1-8 bytes |
| `Float32`, `Float64` | IEEE 754 floating point | 4-8 bytes |
| `Bool` | Boolean value | 1 byte |
| `Str` | UTF-8 string | Variable |
| `Bytes` | Raw byte sequence | Variable |
| `Duration` | Time interval | 8 bytes |
| `Timestamp` | UTC instant | 8 bytes |
| `Hash` | Content-addressable reference | 32 bytes |
| `Confidence` | Probability value [0.0, 1.0] | 4 bytes |
| `AgentId` | Unique agent identifier | 16 bytes |
| `TaskId` | Unique task identifier | 16 bytes |

### 3.3 Composite Types

#### 3.3.1 Schema Types

Schemas define structured data with built-in validation:

```
SCHEMA User => {
    name: Str :: length(1..200),
    email: Str :: pattern(EMAIL_REGEX),
    age: UInt8 :: range(0..150),
    role: ENUM(admin, editor, viewer),
    created: Timestamp
}
```

#### 3.3.2 Collection Types

| Type | Description | Example |
|---|---|---|
| `List[T]` | Ordered sequence | `List[Int32]` |
| `Set[T]` | Unique unordered elements | `Set[Str]` |
| `Map[K, V]` | Key-value mapping | `Map[Str, User]` |
| `Queue[T]` | FIFO ordered queue | `Queue[Task]` |
| `Graph[N, E]` | Node-edge graph structure | `Graph[User, Follows]` |
| `Tensor[T, ...dims]` | Multi-dimensional array | `Tensor[Float32, 768]` |

### 3.4 Probabilistic Types

The defining innovation of AgentLang's type system is native support for uncertainty. Any type can be wrapped in a probabilistic container:

```
// A classification result with confidence
result: Probable[ENUM(positive, negative, neutral)] => {
    value: positive,
    confidence: ~0.87,
    provenance: SHA256:a3f8...,
    degradation: RETRY(3) -> ESCALATE
}
```

Probabilistic types enforce handling of uncertainty at compile time. An agent cannot use a `Probable[T]` where a `T` is expected without explicitly acknowledging and handling the confidence bound:

```
// Compile error: cannot use Probable[Str] as Str
greeting: Str = llm_generate("Say hello")  // ERROR

// Correct: explicit confidence handling
greeting: Str = llm_generate("Say hello")
    WHEN confidence? >= 0.9 -> USE value
    WHEN confidence? >= 0.7 -> RETRY(2)
    OTHERWISE -> ESCALATE
```

### 3.5 Dependent Types

Types can depend on values, enabling expressive compile-time constraints:

```
// A list that must have exactly N elements
top_results: List[User, length: 10]

// A tensor with specific dimensions
embedding: Tensor[Float32, 768]

// A value constrained by business logic
discount: Float64 :: range(0.0..0.5)  // max 50% discount
```

### 3.6 Type Aliases and Union Types

```
TYPE UserId = UInt64
TYPE FailureDetails = Map[Str, JsonValue] | NONE
TYPE Result[T] = SUCCESS(T) | FAILURE(ErrorCode, message: Str, details: FailureDetails)
TYPE JsonValue = Str | Float64 | Bool | NONE | List[JsonValue] | Map[Str, JsonValue]
```

---

## 4. Execution Model

### 4.1 Dataflow Architecture

AgentLang programs are not sequences of instructions. They are directed acyclic graphs (DAGs) of operations connected by data dependencies. The runtime automatically determines execution order and parallelization from the graph structure.

A simple data pipeline:

```
PIPELINE get_top_customers =>
    SOURCE customers
    -> FILTER status EQ active
    -> FILTER revenue GT 10000
    -> SORT revenue DESC
    -> TAKE 10
    -> EMIT
```

A parallel pipeline with merge:

```
PIPELINE analyze_market =>
    FORK {
        branch_a: SOURCE sales -> REDUCE SUM revenue,
        branch_b: SOURCE costs -> REDUCE SUM amount,
        branch_c: SOURCE forecasts -> FILTER confidence? >= 0.8
    }
    -> MERGE(branch_a, branch_b, branch_c)
    -> MAP {profit: branch_a - branch_b, outlook: branch_c}
    -> EMIT
```

> **NOTE:** FORK branches execute in parallel automatically. The runtime schedules them across available compute resources without explicit thread or async management.

### 4.2 Operations

An operation is the fundamental unit of computation in AgentLang. Every operation is a pure function with declared inputs, outputs, constraints, and verification conditions:

```
OPERATION calculate_discount =>
    INPUT  customer: User, order: Order
    OUTPUT Float64 :: range(0.0..0.5)
    REQUIRE customer.status EQ active
    REQUIRE order.total GT 0
    ENSURE result <= customer.tier.max_discount
    BODY {
        base_rate = customer.tier.discount_rate
        volume_bonus = WHEN order.total GT 1000 -> 0.05, OTHERWISE -> 0.0
        EMIT MIN(base_rate + volume_bonus, customer.tier.max_discount)
    }
```

> **MVP v0.1 normative note (OUTPUT syntax):** The canonical `OUTPUT` clause takes a type expression only: `OUTPUT type_expr` (see `GRAMMAR_MVP.ebnf`). Named output forms (`OUTPUT name: type_expr`) are not permitted. Post-conditions (`ENSURE` clauses) reference the operation result via the implicit binding `result`.

### 4.3 Pipeline Composition

Pipelines compose naturally through the arrow operator. Each stage in a pipeline is an operation that transforms data flowing through it:

```
// Named pipeline composition
PIPELINE process_order =>
    validate_input
    -> calculate_discount
    -> apply_tax
    -> generate_invoice
    -> CHECKPOINT
    -> send_notification
    -> EMIT
```

The `CHECKPOINT` keyword creates a serialization point. If the pipeline fails after the checkpoint, it can be resumed from that point rather than restarting from the beginning.

### 4.4 Conditional Execution

AgentLang replaces traditional if/else chains with pattern matching that ensures exhaustive handling:

```
MATCH customer.tier =>
    WHEN platinum -> apply_premium_discount
    WHEN gold    -> apply_standard_discount
    WHEN silver  -> apply_basic_discount
    WHEN trial   -> EMIT 0.0
    // Compile error if any tier value is unhandled
```

### 4.5 Bounded Iteration

All loops in AgentLang require explicit bounds to prevent infinite execution. An unbounded loop is a compile-time error:

```
// Bounded loop with maximum iterations
LOOP max: 100 =>
    page = fetch_next_page(cursor)
    WHEN page.empty -> HALT(COMPLETE)
    process_page(page)
    cursor = page.next_cursor

// Compile error: no bound specified
LOOP =>  // ERROR: LOOP requires explicit bound
    do_something()
```

### 4.6 Error Handling

AgentLang does not use exceptions. All errors are values that must be explicitly handled through the Result type:

```
result = fetch_user(user_id)
MATCH result =>
    WHEN SUCCESS(user) -> process(user)
    WHEN FAILURE(NOT_FOUND, msg, details) -> create_default_user()
    WHEN FAILURE(TIMEOUT, msg, details) -> RETRY(3, backoff: exponential)
    WHEN FAILURE(_, msg, details) -> ESCALATE(msg)
```

---

## 5. Memory Model

### 5.1 Content-Addressable Storage

AgentLang uses a content-addressable storage (CAS) model inspired by Git's object store. Every value is stored by the hash of its content. This provides several critical properties for agent operations:

- **Automatic deduplication:** Identical values are stored once regardless of how many references exist.
- **Built-in integrity verification:** Any data corruption is immediately detectable by hash mismatch.
- **Safe concurrent access:** Immutable-by-default values eliminate race conditions.
- **Perfect audit trails:** Every value's provenance is traceable through its hash chain.

### 5.2 Named References

While all values are stored by hash, agents interact with data through named `STORE` references that point to specific hashes:

```
STORE customers = LOAD("db://main/customers")
// Internally: customers -> SHA256:7f3a...

// Mutation creates a new hash, old data is preserved
STORE updated = customers -> FILTER status EQ active
// updated -> SHA256:b2c1... (new hash, new data)
// customers -> SHA256:7f3a... (unchanged)
```

### 5.3 Mutability

Immutability is the default. Mutable bindings require explicit declaration with a justification annotation that aids verification and debugging:

```
// Immutable (default) - any reassignment is a compile error
STORE config = load_config()

// Mutable - requires justification
MUTABLE cursor @reason("pagination state across loop iterations")
cursor = initial_cursor()
LOOP max: 1000 =>
    cursor = process_and_advance(cursor)
    WHEN cursor.done -> HALT(COMPLETE)
```

### 5.4 Memory Scoping

Memory is scoped at three levels:

| Scope | Lifetime | Visibility | Use Case |
|---|---|---|---|
| `LOCAL` | Current operation | Current operation only | Temporary computation |
| `TASK` | Current task/pipeline | All operations in task | Shared pipeline state |
| `AGENT` | Agent lifetime | Current agent only | Agent-persistent configuration |
| `SHARED` | Explicit lifecycle | Specified agent set | Multi-agent coordination |
| `GLOBAL` | System lifetime | All agents | System configuration, registries |

---

## 6. Agent Model

### 6.1 Agent Declaration

An agent is a first-class entity in AgentLang with declared identity, capabilities, and behavioral constraints:

```
AGENT data_processor =>
    CAPABILITIES [DB_READ, DB_WRITE, API_CALL]
    DENY [FILE_SYSTEM, NETWORK_RAW, SELF_MODIFY]
    TRUST_LEVEL ~0.92
    MAX_CONCURRENCY 10
    MEMORY_LIMIT 2GB
    TIMEOUT_DEFAULT 30s
    ON_FAILURE RETRY(3) -> ESCALATE
    STATE_SCHEMA => {
        active_tasks: List[TaskId],
        processed_count: UInt64,
        last_checkpoint: Timestamp
    }
```

### 6.2 Capability System

Capabilities are fine-grained permissions that control what an agent can do. They are checked at compile time where possible and enforced at runtime for dynamic operations:

| Capability | Grants Access To | Risk Level |
|---|---|---|
| `DB_READ` | Read from any registered data source | Low |
| `DB_WRITE` | Write to any registered data source | Medium |
| `API_CALL` | Make outbound HTTP/gRPC requests | Medium |
| `FILE_READ` | Read from filesystem | Medium |
| `FILE_WRITE` | Write to filesystem | High |
| `NETWORK_RAW` | Raw TCP/UDP socket access | High |
| `AGENT_SPAWN` | Create new agent instances | High |
| `SELF_MODIFY` | Modify own code/operations | Critical |
| `ESCALATE_HUMAN` | Request human intervention | Low |
| `CRYPTO_SIGN` | Sign data with agent's private key | Critical |

### 6.3 Delegation

> **MVP v0.1 override:** Delegation executes under callee's own capabilities, not caller's. Caller must hold `DELEGATE` capability. No implicit capability inheritance or intersection is permitted. See `MVP_PROFILE.md` §8.

Delegation is the primary coordination mechanism between agents. It transfers responsibility for a task to another agent with explicit policies:

```
DELEGATE process_batch TO data_processor =>
    INPUT batch_data
    TIMEOUT 5m
    ON_TIMEOUT RETRY(1) -> REASSIGN(backup_processor) -> ABORT
    SHARED_CONTEXT [config, schema_registry]
    ISOLATION {
        batch_data: READ_ONLY,
        output_store: WRITE_ONLY,
        config: READ_ONLY
    }
    ON_COMPLETE validate_results -> merge_output
    ON_FAILURE log_error -> ESCALATE
```

### 6.4 Agent Communication

Agents communicate through typed message channels. Messages are validated against their schema at send time:

```
CHANNEL task_updates: Queue[TaskUpdate] =>
    MAX_SIZE 1000
    OVERFLOW_POLICY DROP_OLDEST
    SUBSCRIBERS [monitor_agent, dashboard_agent]

// Sending
EMIT TO task_updates => TaskUpdate {
    task_id: current_task,
    status: COMPLETED,
    metrics: {duration: 2.3s, records: 15000}
}

// Receiving (reactive)
OBSERVE task_updates =>
    WHEN status EQ FAILED -> trigger_alert
    WHEN duration GT 10s -> log_slow_task
```

### 6.5 Trust and Verification

Every agent has a trust level that affects how its outputs are treated by other agents:

```
// High-trust agent output can be used directly
result = DELEGATE analyze TO trusted_agent  // trust ~0.95
// result.confidence inherits agent trust

// Low-trust agent output requires verification
result = DELEGATE analyze TO new_agent  // trust ~0.60
REQUIRE VERIFY(result) BY trusted_agent
    WHEN verified -> USE result
    WHEN rejected -> DISCARD -> ESCALATE
```

### 6.6 Agent Lifecycle

| State | Description | Transitions To |
|---|---|---|
| `INITIALIZING` | Loading state and validating capabilities | `READY`, `FAILED` |
| `READY` | Available to accept tasks | `EXECUTING`, `SUSPENDED` |
| `EXECUTING` | Actively processing a task | `READY`, `CHECKPOINTED`, `FAILED` |
| `CHECKPOINTED` | State serialized, execution paused | `EXECUTING`, `SUSPENDED` |
| `SUSPENDED` | Temporarily inactive, state preserved | `READY`, `TERMINATED` |
| `FAILED` | Unrecoverable error | `INITIALIZING`, `TERMINATED` |
| `TERMINATED` | Permanently stopped | None (terminal state) |

---

## 7. Verification System

### 7.1 Philosophy

Verification is not an optional add-on in AgentLang; it is woven into the language's fabric. Every operation carries proof obligations that must be discharged before execution. This is essential for AI agents that may hallucinate, generate incorrect logic, or produce unexpected side effects.

### 7.2 Pre-conditions and Post-conditions

Every operation declares what must be true before it runs (`REQUIRE`) and what it guarantees after completion (`ENSURE`):

```
OPERATION transfer_funds =>
    INPUT from: Account, to: Account, amount: Float64
    OUTPUT TransferReceipt
    REQUIRE amount GT 0
    REQUIRE from.balance GTE amount
    REQUIRE from.id NEQ to.id
    ENSURE from.balance EQ OLD(from.balance) - amount
    ENSURE to.balance EQ OLD(to.balance) + amount
    ENSURE result.status EQ COMPLETED
    INVARIANT from.balance + to.balance EQ OLD(from.balance) + OLD(to.balance)
```

> **NOTE:** The `OLD()` function references the value before the operation executed, enabling relational post-conditions.

### 7.3 Verification Levels

AgentLang supports three levels of verification, each with increasing cost and assurance:

| Level | Mechanism | Cost | Assurance |
|---|---|---|---|
| `ASSERT` | Runtime check (fails immediately) | Low | Current execution only |
| `PROVE_STATIC` | Compile-time formal proof (SMT solver) | Medium | All possible executions |
| `PROVE_RUNTIME` | Runtime proof with evidence trail | High | Current execution with audit log |

### 7.4 Self-Modification Verification

When an agent with `SELF_MODIFY` capability proposes changes to its own operations, the verification system enforces a strict protocol:

```
MUTATE OPERATION calculate_discount =>
    PROPOSED_CHANGE {
        // New implementation here
    }
    PROOF {
        preconditions_preserved: PROVE_STATIC,
        postconditions_preserved: PROVE_STATIC,
        no_new_capabilities_required: PROVE_STATIC,
        performance_regression: PROVE_RUNTIME <= 10%,
        all_tests_pass: PROVE_RUNTIME
    }
    APPROVAL AUTO WHEN proof.all_pass
    APPROVAL ESCALATE WHEN proof.any_fail
```

---

## 8. Standard Library

### 8.1 Core Modules

| Module | Purpose | Key Operations |
|---|---|---|
| `core.data` | Data transformation primitives | FILTER, MAP, REDUCE, SORT, GROUP |
| `core.io` | Input/output operations | READ, WRITE, FETCH, STREAM |
| `core.math` | Mathematical operations | Standard arithmetic, linear algebra, statistics |
| `core.text` | Text processing | PARSE, FORMAT, REGEX, TOKENIZE, EMBED |
| `core.time` | Temporal operations | NOW, DURATION, SCHEDULE, INTERVAL |
| `core.crypto` | Cryptographic operations | HASH, SIGN, VERIFY, ENCRYPT, DECRYPT |
| `core.json` | JSON serialization | ENCODE, DECODE, VALIDATE, PATCH |
| `core.http` | HTTP client operations | GET, POST, PUT, DELETE with retry policies |

### 8.2 Agent Modules

| Module | Purpose | Key Operations |
|---|---|---|
| `agent.llm` | LLM interaction primitives | GENERATE, CLASSIFY, EXTRACT, SUMMARIZE |
| `agent.memory` | Agent memory management | REMEMBER, RECALL, FORGET, SEARCH |
| `agent.tools` | External tool integration | REGISTER_TOOL, INVOKE, DISCOVER |
| `agent.planning` | Task planning and decomposition | PLAN, DECOMPOSE, PRIORITIZE, REPLAN |
| `agent.reflection` | Self-analysis and improvement | EVALUATE, CRITIQUE, IMPROVE, LEARN |

### 8.3 Integration Modules

| Module | Purpose | Key Operations |
|---|---|---|
| `db.sql` | SQL database interaction | QUERY, INSERT, UPDATE, DELETE, MIGRATE |
| `db.vector` | Vector database operations | UPSERT, SIMILARITY_SEARCH, CLUSTER |
| `db.graph` | Graph database operations | TRAVERSE, SHORTEST_PATH, PATTERN_MATCH |
| `api.rest` | REST API construction | ENDPOINT, MIDDLEWARE, VALIDATE, RESPOND |
| `api.grpc` | gRPC service definitions | SERVICE, METHOD, STREAM_IN, STREAM_OUT |
| `queue.pubsub` | Message queue operations | PUBLISH, SUBSCRIBE, ACK, NACK, REPLAY |

### 8.4 LLM Integration

The `agent.llm` module provides first-class integration with large language models, with built-in confidence tracking and structured output:

```
OPERATION classify_intent =>
    INPUT message: Str
    OUTPUT Probable[ENUM(question, command, feedback, other)]
    BODY {
        result = LLM.CLASSIFY {
            model: "claude-sonnet-4-20250514",
            input: message,
            categories: [question, command, feedback, other],
            min_confidence: ~0.80
        }
        WHEN result.confidence? < 0.80 -> RETRY(2, rephrase: TRUE)
        EMIT result
    }
```

---

## 9. State Management and Checkpointing

### 9.1 Automatic Checkpointing

Agent execution is inherently interruptible. Network failures, timeouts, rebalancing, and deliberate pauses all require state preservation. AgentLang makes checkpointing a language-level primitive rather than an application-level concern.

```
PIPELINE long_running_etl =>
    SOURCE large_dataset
    -> BATCH(size: 1000)
    -> MAP transform_record
    -> CHECKPOINT @every(100 batches)  // automatic periodic checkpoint
    -> LOAD target_db
    -> EMIT summary
```

### 9.2 State Serialization

All agent state is serializable by construction. The type system ensures that every value in an agent's state can be serialized, transmitted, and deserialized without loss:

```
// Checkpoint captures complete execution state
CHECKPOINT =>
    execution_point: CURRENT_POSITION,
    local_state: SERIALIZE(all_local_bindings),
    pipeline_progress: {batch: 347, record: 42},
    hash: SHA256(above),
    timestamp: NOW()

// Resume from checkpoint
RESUME FROM checkpoint_hash =>
    REQUIRE VERIFY_INTEGRITY(checkpoint_hash)
    RESTORE local_state
    CONTINUE FROM execution_point
```

### 9.3 State Migration

When an agent's code is updated (through self-modification or external deployment), checkpoints may need migration. AgentLang supports declarative state migration:

```
MIGRATE STATE v1 -> v2 =>
    ADD_FIELD processed_count: UInt64 DEFAULT 0
    RENAME_FIELD old_name -> new_name
    DROP_FIELD deprecated_field
    TRANSFORM custom_field => custom_field * 1000  // unit conversion
    PROVE postconditions_preserved
```

---

## 10. Concurrency Model

### 10.1 Implicit Parallelism

AgentLang's dataflow execution model provides implicit parallelism. The runtime analyzes the dependency graph and automatically schedules independent operations on available compute resources:

```
PIPELINE analyze =>
    // These three operations have no data dependencies
    // Runtime executes them in parallel automatically
    revenue = SOURCE sales -> REDUCE SUM amount
    costs = SOURCE expenses -> REDUCE SUM amount
    headcount = SOURCE employees -> FILTER active -> COUNT

    // This depends on all three above - waits automatically
    EMIT {revenue, costs, headcount, profit: revenue - costs}
```

### 10.2 Explicit Fork/Join

For cases requiring explicit parallel execution with controlled merge semantics:

```
FORK {
    api_a: FETCH "https://service-a/data" TIMEOUT 5s,
    api_b: FETCH "https://service-b/data" TIMEOUT 5s,
    api_c: FETCH "https://service-c/data" TIMEOUT 5s
}
JOIN strategy: BEST_EFFORT  // continue with whatever completes
    WHEN ALL_COMPLETE -> merge_all
    WHEN PARTIAL(min: 2) -> merge_available
    WHEN TIMEOUT(10s) -> use_cached_fallback
```

> **MVP v0.1 override:** Only `JOIN strategy: ALL_COMPLETE` is permitted. `BEST_EFFORT` and `PARTIAL(min=k)` are excluded from MVP v0.1 and must be rejected at compile time with `NOT_IMPLEMENTED`. See `MVP_PROFILE.md` §2.

### 10.3 Resource Locking

When mutable shared state is unavoidable, AgentLang provides explicit resource locking with deadlock prevention:

```
ACQUIRE lock ON shared_counter TIMEOUT 5s =>
    shared_counter = shared_counter + batch_size
    // RELEASE is automatic at scope exit

// Deadlock prevention: locks must be acquired in declared order
ACQUIRE lock ON [resource_a, resource_b] ORDER(a, b) =>
    // Runtime enforces ordering, preventing circular waits
```

---

## 11. Interoperability

### 11.1 Foreign Function Interface (FFI)

AgentLang provides a foreign function interface for calling into existing language ecosystems. All FFI calls are capability-gated and automatically wrapped in probabilistic types:

```
FFI python =>
    IMPORT numpy AS np
    IMPORT pandas AS pd

OPERATION analyze_dataframe =>
    INPUT data: List[Map[Str, JsonValue]]
    OUTPUT Probable[Map[Str, Float64]]
    REQUIRE CAPABILITY API_CALL
    BODY {
        df = FFI.python => pd.DataFrame(data)
        stats = FFI.python => df.describe().to_dict()
        EMIT stats WITH confidence: ~0.99  // deterministic FFI
    }
```

### 11.2 API Generation

AgentLang operations can be automatically exposed as REST or gRPC APIs:

```
EXPOSE get_top_customers AS REST =>
    PATH "/api/v1/customers/top"
    METHOD GET
    AUTH JWT
    RATE_LIMIT 100/min
    CACHE TTL 5m
    RESPONSE_SCHEMA List[Customer, length: 1..100]
```

### 11.3 Event Integration

Integration with external event systems (Kafka, RabbitMQ, webhooks) is declarative:

```
SUBSCRIBE kafka://orders.created =>
    SCHEMA OrderEvent
    GROUP consumer_group_1
    OFFSET latest
    ON_MESSAGE order_event ->
        validate_order
        -> process_payment
        -> update_inventory
        -> ACK
    ON_ERROR NACK -> RETRY(3) -> DEAD_LETTER
```

---

## 12. Tooling and Ecosystem

### 12.1 Compiler Architecture

The AgentLang compiler operates in four phases:

| Phase | Input | Output | Key Operations |
|---|---|---|---|
| Parse | Source text | Abstract Syntax Tree (AST) | Tokenization, syntax validation |
| Verify | AST | Verified AST | Type checking, proof obligation discharge, capability validation |
| Optimize | Verified AST | Optimized DAG | Dead code elimination, operation fusion, parallelism discovery |
| Emit | Optimized DAG | Executable bytecode | Code generation targeting AgentLang VM |

### 12.2 Runtime Environment

The AgentLang runtime (ALVM — AgentLang Virtual Machine) provides:

- **Dataflow scheduler:** Analyzes operation dependency graphs and schedules parallel execution across available compute resources.
- **Checkpoint manager:** Handles automatic and manual state serialization with configurable storage backends.
- **Capability enforcer:** Runtime enforcement of capability restrictions that cannot be fully verified at compile time.
- **Agent registry:** Manages agent lifecycle, discovery, and health monitoring.
- **Proof verifier:** Runtime verification engine for `PROVE_RUNTIME` obligations.
- **FFI bridge:** Manages foreign function calls with sandboxing and type marshaling.

### 12.3 Debugging and Observability

Since agents are the primary developers, debugging tools are designed for programmatic consumption:

```
TRACE PIPELINE process_order =>
    CAPTURE {
        inputs: ALL,
        outputs: ALL,
        intermediate_values: SAMPLED(10%),
        timing: PER_OPERATION,
        confidence_changes: ALL
    }
    FORMAT structured_json
    SINK observability_store
```

### 12.4 Package System

AgentLang packages are content-addressed bundles of operations with verified interfaces:

```
PACKAGE ecommerce.pricing v2.1.0 =>
    EXPORTS [calculate_discount, apply_tax, generate_invoice]
    REQUIRES [core.data v1.0+, core.math v1.0+]
    CAPABILITIES_NEEDED [DB_READ]
    HASH SHA256:e4f2...
    VERIFIED_BY [auditor_agent_1, auditor_agent_2]
    LICENSE MIT
```

---

## 13. Security Model

### 13.1 Principle of Least Privilege

Every agent operates with the minimum capabilities required for its task. Capabilities are declared at agent definition time and cannot be escalated without explicit approval from a higher-authority agent or human operator.

### 13.2 Sandboxing

All operations execute in sandboxed environments with strict resource limits:

```
SANDBOX task_execution =>
    MEMORY_LIMIT 512MB
    CPU_LIMIT 2 cores
    NETWORK restricted_to: [api.internal.com, db.internal.com]
    FILESYSTEM NONE
    MAX_DURATION 5m
    ON_LIMIT_EXCEEDED TERMINATE -> LOG -> ALERT
```

### 13.3 Audit Trail

Every state change, capability usage, and inter-agent communication is logged in an append-only, cryptographically signed audit trail:

```
AUDIT_ENTRY =>
    timestamp: 2026-02-22T10:30:00Z,
    agent: data_processor,
    action: DB_WRITE,
    target: customers_table,
    records_affected: 150,
    verification: PASSED,
    proof_hash: SHA256:c7d1...,
    previous_entry: SHA256:b3a2...
```

### 13.4 Human Override

The security model includes mandatory human escalation points for critical operations:

```
OPERATION delete_production_data =>
    REQUIRE CAPABILITY DB_WRITE
    REQUIRE HUMAN_APPROVAL =>
        APPROVER role: admin
        TIMEOUT 24h
        ON_TIMEOUT ABORT
        EVIDENCE {reason, impact_analysis, rollback_plan}
```

---

## 14. Complete Example Program

The following is a complete AgentLang program demonstrating a multi-agent system that processes customer orders, detects fraud, and generates reports:

```
// ═══════════════════════════════════════════════════
// Order Processing System - AgentLang v1.0
// ═══════════════════════════════════════════════════

// ── Schema Definitions ──
SCHEMA Order => {
    id: UInt64,
    customer_id: UInt64,
    items: List[OrderItem, length: 1..1000],
    total: Float64 :: range(0.01..1000000.00),
    status: ENUM(pending, verified, shipped, delivered, cancelled),
    created: Timestamp
}

SCHEMA FraudSignal => {
    order_id: UInt64,
    risk_score: Confidence,
    indicators: List[Str],
    recommendation: ENUM(approve, review, reject)
}

// ── Agent Definitions ──
AGENT order_processor =>
    CAPABILITIES [DB_READ, DB_WRITE, API_CALL]
    TRUST_LEVEL ~0.90
    MAX_CONCURRENCY 50

AGENT fraud_detector =>
    CAPABILITIES [DB_READ, API_CALL]
    TRUST_LEVEL ~0.85
    MAX_CONCURRENCY 20

AGENT report_generator =>
    CAPABILITIES [DB_READ, FILE_WRITE]
    TRUST_LEVEL ~0.92
    MAX_CONCURRENCY 5

// ── Core Operations ──
OPERATION validate_order =>
    INPUT order: Order
    OUTPUT Result[Order]
    REQUIRE order.items.length GT 0
    REQUIRE order.total GT 0
    ENSURE result.status EQ verified OR result IS FAILURE
    BODY {
        stock_check = order.items
            -> MAP item => check_inventory(item.product_id, item.quantity)
            -> FILTER available EQ FALSE
        WHEN stock_check.length GT 0
            -> EMIT FAILURE(
                OUT_OF_STOCK,
                "One or more order items are unavailable",
                {unavailable_items: stock_check}
            )
        OTHERWISE
            -> EMIT SUCCESS(order WITH status: verified)
    }

OPERATION assess_fraud =>
    INPUT order: Order, customer_history: List[Order]
    OUTPUT Probable[FraudSignal]
    BODY {
        indicators = FORK {
            velocity: check_order_velocity(customer_history),
            amount: check_unusual_amount(order, customer_history),
            location: check_geo_anomaly(order)
        }
        -> JOIN strategy: ALL_COMPLETE
        -> FLATTEN

        risk_score = LLM.CLASSIFY {
            model: "claude-sonnet-4-20250514",
            input: {order, indicators},
            output_type: Confidence,
            min_confidence: ~0.75
        }

        recommendation = MATCH risk_score =>
            WHEN GT ~0.8 -> reject
            WHEN GT ~0.5 -> review
            OTHERWISE    -> approve

        EMIT FraudSignal {
            order_id: order.id,
            risk_score, indicators, recommendation
        }
    }

// ── Main Pipeline ──
PIPELINE process_orders @entry =>
    SOURCE SUBSCRIBE kafka://orders.new
    -> BATCH(size: 100, timeout: 5s)
    -> MAP order =>
        DELEGATE validate_order TO order_processor
            INPUT order
            TIMEOUT 10s
    -> FILTER result IS SUCCESS
    -> MAP order =>
        history = QUERY db://customers WHERE id EQ order.customer_id
        DELEGATE assess_fraud TO fraud_detector
            INPUT {order, customer_history: history}
            TIMEOUT 30s
    -> CHECKPOINT @every(10 batches)
    -> FORK {
        approved: FILTER recommendation EQ approve
            -> MAP finalize_order,
        flagged: FILTER recommendation EQ review
            -> MAP flag_for_review
            -> ESCALATE TO human_reviewer,
        rejected: FILTER recommendation EQ reject
            -> MAP cancel_order -> notify_customer
    }
    -> EMIT summary
```

---

## 15. Formal Grammar (EBNF)

The following is a simplified Extended Backus-Naur Form (EBNF) grammar for AgentLang's core syntax:

```ebnf
program        = { declaration } ;
declaration    = schema_decl | agent_decl | operation_decl
               | pipeline_decl | type_decl | package_decl ;

schema_decl    = "SCHEMA" identifier "=>" "{" { field_decl } "}" ;
field_decl     = identifier ":" type_expr [ "::" constraint ] ;

agent_decl     = "AGENT" identifier "=>"
                 { agent_property } ;
agent_property = "CAPABILITIES" "[" cap_list "]"
               | "DENY" "[" cap_list "]"
               | "TRUST_LEVEL" confidence_lit
               | "MAX_CONCURRENCY" integer_lit
               | "MEMORY_LIMIT" size_lit
               | "TIMEOUT_DEFAULT" duration_lit
               | "ON_FAILURE" failure_policy
               | "STATE_SCHEMA" "=>" "{" { field_decl } "}" ;

operation_decl = "OPERATION" identifier "=>"
                 { "INPUT" param_list }
                 { "OUTPUT" type_expr }
                 { require_clause }
                 { ensure_clause }
                 { invariant_clause }
                 "BODY" "{" { statement } "}" ;

pipeline_decl  = "PIPELINE" identifier [ annotations ] "=>"
                 source_clause
                 { "->" pipe_stage }
                 "->" "EMIT" [ expression ] ;

pipe_stage     = operation_ref | filter_expr | sort_expr
               | batch_expr | checkpoint_expr | fork_expr ;

fork_expr      = "FORK" "{" { branch } "}"
                 [ "->" join_expr ] ;
branch         = identifier ":" pipe_stage { "->" pipe_stage } ;
join_expr      = "JOIN" "strategy:" join_strategy ;

type_expr      = primitive_type | composite_type
               | probable_type | union_type | alias_ref ;
probable_type  = "Probable" "[" type_expr "]" ;
composite_type = collection_type | schema_ref ;
collection_type= ( "List" | "Set" | "Map" | "Queue" )
                 "[" type_expr { "," type_expr } "]"
                 [ "," constraint ] ;

expression     = literal | identifier | operation_call
               | pipe_expr | match_expr | fork_expr ;
pipe_expr      = expression "->" expression ;
match_expr     = "MATCH" expression "=>"
                 { "WHEN" pattern "->" expression } ;

delegate_expr  = "DELEGATE" identifier "TO" identifier
                 "=>" { delegate_clause } ;
delegate_clause= "INPUT" expression
               | "TIMEOUT" duration_lit
               | "ON_TIMEOUT" failure_policy
               | "SHARED_CONTEXT" "[" id_list "]"
               | "ISOLATION" "{" { isolation_rule } "}" ;

annotations    = { "@" identifier [ "(" arg_list ")" ] } ;
constraint     = identifier "(" arg_list ")" ;
failure_policy = policy_step { "->" policy_step } ;
policy_step    = "RETRY" "(" integer_lit [ "," retry_opts ] ")"
               | "REASSIGN" "(" identifier ")"
               | "ESCALATE" [ "(" expression ")" ]
               | "ABORT" ;
```

---

## 16. Comparison with Existing Languages

| Feature | AgentLang | Python | Rust | Erlang/Elixir |
|---|---|---|---|---|
| Token Efficiency | Very High | Medium | Low | Medium |
| Formal Verification | Built-in | None | Partial (borrow checker) | None |
| Concurrency Model | Dataflow (implicit) | GIL + async | Ownership + threads | Actor model |
| Probabilistic Types | Native | None | None | None |
| Agent Coordination | Language primitive | Library (LangChain) | Library | OTP behaviors |
| State Checkpointing | Built-in | External (pickle) | External | Process state |
| Error Handling | Result types + MATCH | Exceptions | Result\<T,E\> | Pattern matching |
| Self-Modification | Verified mutations | Dynamic (unsafe) | Not supported | Hot code reload |
| Ecosystem Size | New (growing) | Massive | Growing | Moderate |
| Learning Curve | Medium (for agents) | Low (for humans) | High | Medium |

---

## 17. Future Directions

### 17.1 Evolutionary Op-Code Extension

As agents compose operations into higher-level patterns, these compositions can be registered as new first-class operations in the standard library. Over time, the language evolves organically as agents discover and formalize recurring patterns.

### 17.2 Multi-Model Support

Future versions will support heterogeneous agent architectures where agents may be backed by different AI models (LLMs, reinforcement learning agents, symbolic reasoning engines) while communicating through the unified AgentLang type system.

### 17.3 Distributed Execution

The dataflow execution model naturally extends to distributed computing. Future runtime versions will support transparent distribution of pipeline stages across multiple machines with automatic data locality optimization.

### 17.4 Formal Methods Integration

Deeper integration with formal verification tools such as SMT solvers (Z3), theorem provers (Lean, Coq), and model checkers (TLA+) will enable stronger guarantees for critical agent operations.

### 17.5 Natural Language Bridge

A bidirectional translation layer between natural language specifications and AgentLang code will allow human operators to express intent in natural language and receive verified AgentLang programs, while agents can explain their programs in human-readable form for audit purposes.

---

*End of Specification*

*AgentLang v1.0 Draft — February 2026*
