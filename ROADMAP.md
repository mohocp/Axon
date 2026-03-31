# AgentLang Roadmap

This roadmap is organized around the philosophical principles that drive AgentLang's design. Each phase advances a core conviction — not just a feature list, but a deepening of the language's commitment to verification, safety, probabilistic awareness, and agent-native computation.

**Current state:** v0.1.0-rc3 (MVP) — a complete pipeline from source to execution with 22 capabilities, 21 stdlib operations, 6-pass type checker, stub verification, sequential runtime, and full conformance (C1-C20).

---

## Phase 1: Trust the Proof, Not the Agent

**Principle:** Verification by Construction

The MVP stub solver returns `Unknown` for all non-trivial conditions and falls back to runtime assertions. This is the single largest gap between the language's philosophy and its implementation. A language that claims "programs carry their own correctness proofs" must actually prove things.

### 1.1 Real SMT Solver Integration

Integrate Z3 or CVC5 as the verification backend. The stub solver architecture (`StubSolver` trait in `al-vc`) was designed for this — replace the stub with a real solver that translates verification conditions into SMT-LIB2 queries.

**What changes:**
- `REQUIRE`, `ENSURE`, `INVARIANT`, and `ASSERT` clauses produce real proofs at compile time
- `Unknown` results become rare instead of universal
- Synthetic runtime assertions are injected only when genuinely undecidable, not as a default

**Why this comes first:** Every subsequent phase depends on verification being real. Probabilistic types need verified confidence thresholds. Self-modification needs proven safety. Capability delegation needs proven non-escalation. Without a real solver, the entire trust model is aspirational.

**Depends on:** Nothing. Foundation for everything else.

### 1.2 Per-Iteration Loop Invariant Checking

The MVP checks `INVARIANT` once at operation level. The formal semantics define per-iteration checking with inductive discharge — implement this.

**What changes:**
- Loop invariants verified at entry and after each iteration boundary
- Inductive proof obligations generated: base case (loop entry) + inductive step (iteration preserves invariant)

### 1.3 Verification Condition Refinement

Improve VC generation to produce tighter, more precise proof obligations:
- Range constraints on arithmetic (`Int64 :: range(0, 100)`) translated to SMT assertions
- Collection size constraints (`List[T, length: n]`) as solver-checkable bounds
- Pipeline type propagation constraints as chained implications

**Milestone:** A program with `REQUIRE x GT 0` and `ENSURE result GT x` gets a compile-time proof — not a runtime check.

---

## Phase 2: Uncertainty is Not Optional

**Principle:** Probabilistic Awareness

The spec defines `Probable[T]` as a first-class type carrying confidence, provenance, and degradation policy. The MVP has none of this. A language designed for agents that generate probabilistic outputs must make uncertainty mechanically enforced.

### 2.1 Probable[T] Type Implementation

Implement the full probabilistic type:
- `Probable[T]` wraps any value with confidence score `c ∈ [0.0, 1.0]`
- Confidence queries via `?` operator: `result? >= 0.9`
- Compile-time enforcement: `Probable[T]` cannot be used where `T` is expected without explicit threshold handling
- Pattern: `WHEN confidence? >= threshold -> USE value | RETRY(n) | ESCALATE`

### 2.2 Confidence Composition Algebra

Define and implement the rules for how confidence propagates:
- Pipeline composition: `c_pipeline = c_1 * c_2 * ... * c_n` (conservative multiplication)
- Agent trust attenuation: `c_effective = c_value * trust_level_agent`
- Fork/join aggregation: confidence of merged result from branch confidences
- Formalize independence assumptions and calibration method

### 2.3 Provenance Chains

Every `Probable[T]` value carries a provenance hash chain — a cryptographic trail of how the confidence was derived:
- Which operation produced it
- Which agent executed it
- What inputs contributed
- What model/parameters were used (for LLM operations)

### 2.4 Degradation Policies

Attach explicit degradation strategies to probabilistic values:
- What happens when confidence drops below threshold
- Retry policies, escalation chains, fallback values
- Compile-time verification that all degradation paths are handled

**Milestone:** `GENERATE` returns `Probable[Str]` with confidence `~0.85`. The type checker rejects code that uses this as `Str` without checking `result? >= threshold`. Provenance is traceable through the audit trail.

---

## Phase 3: Parallelism Earns Its Name

**Principle:** Parallel by Default

The MVP executes `FORK` branches sequentially. The spec promises "runtime analyzes the dependency graph and automatically schedules independent operations." This phase makes that real.

### 3.1 Concurrent Fork/Join Runtime

Replace sequential branch execution with actual concurrency:
- Thread pool or async runtime for branch execution
- `ALL_COMPLETE` strategy: wait for all branches, collect results
- Proper cancellation semantics for timed-out branches
- Lock-free result collection

### 3.2 BEST_EFFORT and PARTIAL Join Strategies

Implement the advanced join strategies excluded from MVP:
- `BEST_EFFORT`: continue with whatever branches complete within timeout
- `PARTIAL(min: k)`: require at least k of n branches to complete
- Timeout handling per-branch and per-join
- Failure accumulation and compensation logic for cancelled branches

### 3.3 Dataflow DAG Parallelism

Beyond explicit `FORK`, analyze pipeline stages for implicit parallelism:
- Independent pipeline stages execute concurrently
- Operation dependency analysis determines execution order
- Automatic scheduling based on the dataflow DAG

### 3.4 Resource-Aware Scheduling

Agent-declared resource limits (`MAX_CONCURRENCY`, `MEMORY_LIMIT`, `TIMEOUT_DEFAULT`) become runtime scheduling constraints:
- Respect per-agent concurrency caps
- Memory pressure triggers backpressure
- Timeout enforcement with clean cancellation

**Milestone:** A `FORK` with 4 branches hitting 4 different APIs executes in wall-clock time of the slowest branch, not the sum of all four. `BEST_EFFORT` returns partial results when some branches timeout.

---

## Phase 4: Agents Earn Real Capabilities

**Principle:** Agent-Native Coordination + Trust, Safety, Accountability

The MVP has 22 capabilities enforced at runtime and stub backends for I/O, HTTP, and LLM. Agents can't actually *do* anything yet. This phase connects the capability system to the real world.

### 4.1 Real I/O Backends

Replace stub implementations with trait-based pluggable backends:
- `core.io` — real filesystem READ/WRITE/FETCH
- `core.http` — real HTTP GET/POST (+ PUT/DELETE)
- `core.text` — real PARSE/FORMAT/REGEX/TOKENIZE
- Backend injection through runtime configuration, not hardcoded

### 4.2 LLM Provider Integration

Replace `agent.llm` stubs with real LLM backends:
- `GENERATE`, `CLASSIFY`, `EXTRACT` call actual LLM APIs
- Results returned as `Probable[T]` with model-reported confidence
- Provider-agnostic trait: support Claude, GPT, open-source models
- Token usage tracking for resource budgets

### 4.3 Database Modules

Implement the excluded `db.*` modules:
- `db.sql` — QUERY, INSERT, UPDATE, DELETE, MIGRATE
- `db.vector` — UPSERT, SIMILARITY_SEARCH, CLUSTER
- `db.graph` — TRAVERSE, SHORTEST_PATH, PATTERN_MATCH
- Each operation capability-gated (`DB_READ`, `DB_WRITE`)

### 4.4 Queue and Messaging

Implement `queue.pubsub`:
- PUBLISH, SUBSCRIBE, ACK, NACK, REPLAY
- Requires `QUEUE_PUBLISH` / `QUEUE_SUBSCRIBE` capabilities
- Integration patterns for Kafka, RabbitMQ, etc.

### 4.5 Remaining Core Modules

- `core.math` — arithmetic, linear algebra, statistics
- `core.time` — NOW, DURATION, SCHEDULE, INTERVAL
- `core.crypto` — HASH, SIGN, VERIFY, ENCRYPT, DECRYPT
- `core.json` — ENCODE, DECODE, VALIDATE, PATCH

### 4.6 Agent Cognition Modules

- `agent.tools` — REGISTER_TOOL, INVOKE, DISCOVER (external tool integration)
- `agent.planning` — PLAN, DECOMPOSE, PRIORITIZE, REPLAN
- `agent.reflection` — EVALUATE, CRITIQUE, IMPROVE, LEARN
- `agent.memory` — extend with SEARCH beyond current REMEMBER/RECALL/FORGET

**Milestone:** An agent with `CAPABILITIES [DB_READ, LLM_INFER, API_CALL]` can query a database, send results to an LLM for analysis, and POST the output to an API — all with real backends, capability enforcement, and audit logging.

---

## Phase 5: Constrain, Don't Trust

**Principle:** Trust, Safety, Accountability (deepened)

The MVP has capabilities and delegation isolation. This phase adds the enforcement layers the spec envisions: sandboxing, resource budgets, cryptographic audit trails, and human escalation workflows.

### 5.1 SANDBOX Construct

Implement full sandboxed execution:
- `MEMORY_LIMIT` — enforced memory ceiling per sandbox
- `CPU_LIMIT` — core allocation constraints
- `NETWORK` — allowlist-based network access control
- `FILESYSTEM` — path-scoped or `NONE` filesystem access
- `MAX_DURATION` — hard timeout with clean termination
- On limit exceeded: `TERMINATE -> LOG -> ALERT` failure policy chain

### 5.2 Resource Budget Tracking

Introduce `BUDGET` scope for cumulative resource constraints:
- LLM token usage caps
- API call count limits
- Wall-clock time budgets
- Memory high-water marks
- Budget exhaustion triggers degradation policies

### 5.3 Cryptographic Audit Trail

Upgrade the JSONL audit output to a cryptographically signed hash chain:
- Each audit entry includes SHA256 of content + hash of previous entry
- Proof hashes link verification results to audit entries
- Tamper detection: any modification breaks the chain
- Pluggable audit sinks (file, database, external service) beyond stdout

### 5.4 Human Escalation Workflows

Implement `ESCALATE_HUMAN` as a full approval protocol:
- `REQUIRE HUMAN_APPROVAL` with approver role, timeout, evidence requirements
- Approval blocking with clean timeout semantics (`ON_TIMEOUT ABORT`)
- Evidence payloads: reason, impact analysis, rollback plan
- Audit trail records approval decisions with approver identity

### 5.5 Formal Capability Non-Escalation Proof

Using the real SMT solver from Phase 1, prove at compile time that:
- Delegation cannot escalate capabilities beyond the callee's declared set
- Sandbox constraints cannot be bypassed through delegation chains
- Resource budgets are monotonically decreasing through delegation depth

**Milestone:** An untrusted agent runs inside a `SANDBOX` with 512MB memory, no filesystem, restricted network. Its LLM calls are budget-capped at 10K tokens. Every action is cryptographically audited. Critical operations require human approval with 24h timeout.

---

## Phase 6: React, Don't Poll

**Principle:** Agent-Native Coordination (reactive extension)

The MVP is entirely imperative. The spec envisions reactive coordination through channels, observation, and broadcast. This phase adds event-driven multi-agent communication.

### 6.1 Channel Type and Runtime

Implement typed message channels:
- `CHANNEL` declaration with schema, max size, overflow policy, subscriber list
- `EMIT TO channel` syntax for channel-directed output
- Typed: messages validated against schema at send time
- Backpressure: overflow policies (DROP_OLDEST, BLOCK, ERROR)

### 6.2 OBSERVE Reactive Subscriptions

Implement state-change observation:
- `OBSERVE channel => WHEN condition -> action`
- Multiple handlers per observation
- Subscription lifecycle management (subscribe, unsubscribe, cleanup)
- Cycle detection: prevent infinite reactive loops through mutable state

### 6.3 BROADCAST Pub/Sub

Implement scope-wide broadcast:
- Send to all agents in scope
- Ordering guarantees (per-sender FIFO)
- Subscriber failure isolation
- ACK/NACK confirmation semantics

### 6.4 STREAM Operator

Implement unbounded data sequences:
- Lazy evaluation with backpressure
- Cancellation semantics
- Resource management for long-lived streams
- Integration with pipeline composition (`->` and `|>`)

**Milestone:** Agent A publishes task updates to a channel. Agents B and C observe the channel reactively. When a task fails, Agent B retries automatically. When retry exhausts, Agent C escalates to human. No polling — pure reactive coordination.

---

## Phase 7: Agents Improve Themselves

**Principle:** Verification by Construction + Evolutionary Development

The spec's most ambitious feature: agents that modify their own operations with formal proof obligations. This requires Phases 1 (real verification) and 5 (audit trails) as foundations.

### 7.1 MUTATE OPERATION

Implement self-modification with mandatory proofs:
- `MUTATE OPERATION name => PROPOSED_CHANGE { ... } PROOF { ... } APPROVAL ...`
- Proof obligations:
  - `preconditions_preserved` — old preconditions still hold after mutation
  - `postconditions_preserved` — old postconditions still hold
  - `no_new_capabilities_required` — mutation doesn't escalate privilege
  - `performance_regression` — bounded overhead (configurable threshold)
  - `all_tests_pass` — verification suite passes
- Requires `SELF_MODIFY` capability

### 7.2 Verified Approval Workflows

- `APPROVAL AUTO WHEN proof.all_pass` — automatic acceptance if all proofs discharge
- `APPROVAL ESCALATE WHEN proof.any_fail` — escalate to higher-authority agent or human
- Full audit trail of mutation: proposed change, proof results, approval decision, approver identity

### 7.3 Evolutionary Op-Code Extension

From Section 17.1 of the spec: agents discover recurring composition patterns and register them as new first-class operations:
- Pattern detection in operation composition
- Formalization of patterns as new operations with contracts
- Registration in standard library with full verification
- Versioned op-code evolution with backward compatibility

**Milestone:** An agent identifies that it repeatedly applies `FILTER -> MAP -> REDUCE` with a specific pattern. It proposes a new operation `AGGREGATE` encapsulating the pattern. The SMT solver proves the new operation preserves all contracts. The operation is registered and available to all agents.

---

## Phase 8: Types That Think Harder

**Principle:** Semantic Density + Verification by Construction (type-level)

The MVP has monomorphic types and simplified dependent constraints. The spec envisions full parametric polymorphism, dependent types, and effect tracking.

### 8.1 Full Polymorphic Type Inference

Implement System F-style parametric polymorphism:
- Generic function definitions with type parameters
- Type parameter constraints
- Implicit type parameter resolution at call sites
- Higher-kinded types for advanced abstractions

### 8.2 Dependent Type Checking

Extend the decidable fragment of dependent types:
- Value-indexed collection types: `List[User, length: 10]`
- Tensor dimensions: `Tensor[Float32, 768]`
- Range-constrained numerics: `Float64 :: range(0.0..1.0)`
- SMT solver verifies constraints at compile time

### 8.3 Effect System Formalization

Make the implicit capability-as-effect system explicit:
- Operations declare effects alongside types
- Effect composition through pipelines and delegation
- Compile-time effect checking: operation cannot produce effects beyond its declared capabilities
- Effect polymorphism for generic operations

### 8.4 Schema Evolution Framework

Formalize schema subtyping and migration:
- Width subtyping: adding fields is safe
- Depth subtyping: narrowing field types is safe
- Optional field semantics
- Backward/forward compatibility rules
- Versioned schema migration paths

**Milestone:** A generic `OPERATION Transform[T, U]` with constraint `T: Filterable, U: Serializable` type-checks at compile time. Schema `UserV2` is proven backward-compatible with `UserV1`. Effect tracking proves the operation cannot perform `DB_WRITE` when only `DB_READ` is declared.

---

## Phase 9: Beyond One Machine

**Principle:** Parallel by Default (distributed extension)

The dataflow DAG model was designed from the start to support distribution. This phase extends the single-machine runtime to transparent multi-node execution.

### 9.1 Distributed Dataflow Scheduling

Extend the DAG scheduler to distribute pipeline stages across nodes:
- Dependency analysis determines distribution opportunities
- Data locality optimization: move computation to data
- Network cost modeling for distribution decisions
- Automatic serialization of values across node boundaries (CAS hashes enable this)

### 9.2 Cross-Machine Checkpoint/Resume

Extend checkpoint semantics to distributed state:
- Distributed snapshot protocols (Chandy-Lamport or equivalent)
- In-flight message handling during checkpoint
- Consistency guarantees across nodes
- Network partition recovery

### 9.3 Agent Placement Optimization

Automatic agent placement across compute resources:
- Match agent capability requirements to node capabilities
- Resource constraint satisfaction (memory, CPU, network access)
- Failure domain distribution for resilience
- Dynamic rebalancing on node failure

### 9.4 Distributed Capability Enforcement

Extend the capability system across node boundaries:
- Capability verification at network boundaries
- Distributed audit trail synchronization
- Cross-node delegation with capability isolation preserved

**Milestone:** A pipeline with 5 stages automatically distributes across 3 nodes. Checkpoint saves distributed state atomically. A node failure triggers automatic resume on a surviving node from the last checkpoint.

---

## Phase 10: The Agent's Own Toolchain

**Principle:** Agents as Primary Developers

The spec states: "debugging tools are designed for programmatic consumption." This phase builds the development tools agents need — not human IDEs, but agent-consumable tooling.

### 10.1 Interactive REPL

Agent-oriented REPL for interactive development:
- Incremental compilation (evaluate expressions without full recompilation)
- State persistence across evaluations
- Structured output (JSON/JSONL) for agent consumption
- Capability scoping per REPL session

### 10.2 Language Server Protocol (LSP)

IDE integration for both human and agent developers:
- Code completion with capability awareness
- Hover information with type, capability requirements, and verification status
- Go-to-definition across crates and stdlib
- Real-time diagnostics as-you-type
- Verification condition status indicators

### 10.3 Structured Debugging

Agent-consumable debugging tools:
- `TRACE PIPELINE` with structured JSON output
- Breakpoint support with programmatic condition specification
- Value inspection at any point in execution
- Time-travel debugging through checkpoint history
- Confidence flow visualization through pipeline stages

### 10.4 Package Manager

AgentLang package registry for operation distribution:
- Content-addressed packages (CAS-based)
- Verified interfaces: packages carry proofs
- Dependency resolution with capability compatibility checking
- Version management with schema evolution rules

### 10.5 Incremental Compilation

Avoid full recompilation on every change:
- File-level change detection
- Cached intermediate representations (tokens, AST, HIR, type info)
- Incremental type checking (re-check only affected declarations)
- Watch mode for continuous development

**Milestone:** An agent uses the REPL to prototype an operation, the LSP to verify its contracts, the debugger to trace a confidence drop through a pipeline, and the package manager to publish the verified operation for other agents to use.

---

## Phase 11: Speak Both Languages

**Principle:** Agents as Primary Developers + Human Accountability

The spec envisions bidirectional translation between natural language and AgentLang. This bridges the gap between human intent and agent execution.

### 11.1 Natural Language to AgentLang

Human operators express intent in natural language; the system generates verified AgentLang:
- Intent parsing from natural language specifications
- Code generation with automatic contract synthesis (REQUIRE/ENSURE)
- Verification of generated code before acceptance
- Iterative refinement: human feedback narrows intent

### 11.2 AgentLang to Natural Language

Agents explain their programs in human-readable form:
- Operation summaries: what it does, what it requires, what it guarantees
- Pipeline narratives: step-by-step explanation of dataflow
- Audit trail explanations: why an agent took a specific action
- Verification result explanations: what was proven and what wasn't

### 11.3 Multi-Model Agent Backends

Support heterogeneous agent architectures:
- LLM-backed agents (language reasoning)
- Reinforcement learning agents (optimization)
- Symbolic reasoning engines (logic, planning)
- Unified type system: all backends communicate through AgentLang types
- Backend declaration in agent definitions

**Milestone:** A human says "analyze customer churn and recommend retention strategies." The system generates a verified AgentLang pipeline with an LLM agent for analysis, a statistical agent for modeling, and a planning agent for strategy generation. Each agent's actions are explainable in natural language through the audit trail.

---

## Phase 12: Prove It Deeper

**Principle:** Verification by Construction (formal methods frontier)

Beyond SMT solving, integrate with theorem provers for the strongest possible guarantees on critical agent operations.

### 12.1 Theorem Prover Integration

- Lean integration for deep mathematical verification
- Coq integration for certified compiler semantics
- TLA+ integration for distributed protocol verification
- Proof certificate generation and independent checking

### 12.2 Certified Compilation

Prove that the compiler preserves semantics:
- Formalize operational semantics in Lean/Coq
- Prove preservation: compiled code behaves identically to source semantics
- Certified optimization passes: transformations proven sound

### 12.3 Automated Tactic Synthesis

Agents generate proofs of their own correctness:
- Tactic libraries for common verification patterns
- Automated proof search for operation contracts
- Proof reuse: similar operations share proof strategies

**Milestone:** A critical financial operation has a Lean proof that it never produces negative balances. The proof is machine-checked, stored in the audit trail, and independently verifiable by any party.

---

## Dependency Map

```
Phase 1: Real Verification (SMT)
    |
    +---> Phase 2: Probabilistic Types (needs verified confidence thresholds)
    |
    +---> Phase 5: Sandboxing & Audit (needs verified non-escalation)
    |         |
    |         +---> Phase 7: Self-Modification (needs verified proofs + audit)
    |
    +---> Phase 8: Advanced Types (needs solver for dependent type constraints)
    |         |
    |         +---> Phase 12: Theorem Provers (extends verification frontier)
    |
    +---> Phase 3: Concurrent Runtime
              |
              +---> Phase 4: Real Backends (benefits from concurrency)
              |         |
              |         +---> Phase 6: Reactive/Channels (needs real I/O + concurrency)
              |
              +---> Phase 9: Distribution (extends concurrency across machines)

Phase 10: Toolchain (independent, can progress in parallel with any phase)
Phase 11: NL Bridge (needs Phase 4 LLM backends + Phase 8 type system)
```

---

## Guiding Constraints

These constraints apply to every phase, derived from the language's philosophy:

1. **No feature ships without verification.** Every new construct must generate verification conditions. If the SMT solver can't handle it, inject runtime assertions. Never silent failure.

2. **Capabilities gate everything.** Every new operation, backend, or integration point must be gated by an appropriate capability. No ambient authority.

3. **Audit everything.** Every new runtime behavior must produce audit events. The audit trail is not optional infrastructure — it's a language guarantee.

4. **Immutable by default.** New features default to immutable semantics. Mutable state requires explicit justification and is tracked through the audit trail.

5. **Agent-first, human-compatible.** New tooling is designed for programmatic consumption first. Human readability is welcome but never at the cost of machine parsability.

6. **Fail-open compile, fail-closed execute.** When static verification is inconclusive, compilation succeeds with runtime checks inserted. Runtime checks halt with full diagnostic context on failure.

7. **Confidence flows, never hides.** Once probabilistic types exist (Phase 2), every operation that introduces uncertainty must return `Probable[T]`. Confidence propagates through pipelines and attenuates through delegation. Hiding uncertainty is a type error.
