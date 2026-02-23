# AgentLang v1.0 — Comprehensive Holistic Review

**Reviewer:** Claude Opus 4.6
**Date:** 2026-02-22
**Documents reviewed:**
- `AgentLang_Specification_v1.0.md` (base spec)
- `formal_semantics.md` (formal semantics)
- `stdlib_spec.md` (standard library API)

---

## Executive Summary

AgentLang presents a genuinely novel design point: a programming language where AI agents are the primary authors and consumers of code. The probabilistic type system, capability-gated execution, and dataflow-first execution model are well-motivated and internally coherent at the conceptual level. However, the specification has significant gaps that would block a conformant implementation. This review identifies **4 architectural gaps**, **12 cross-document contradictions**, **15 undefined edge cases**, and **9 ergonomic concerns** that must be resolved before the language can move from draft to implementable specification.

---

## 1. Architectural Gaps — What Is Missing for an "AI-Native" Language

### 1.1 No Token Budget / Cost Accounting Model

**Severity: Critical**

AgentLang claims to optimize for "semantic density" and targets LLM-backed agents, yet it has no language-level primitive for tracking or constraining token consumption. Every LLM call has a financial and latency cost. An AI-native language should make this a first-class concern:

- No `TOKEN_BUDGET` scope or constraint annotation.
- No way to express "this pipeline must complete within N tokens of LLM usage."
- The `agent.llm.GENERATE` stdlib operation accepts `max_tokens` but there is no aggregation mechanism across a pipeline or delegation chain.
- The `RETRY` mechanism can silently multiply token costs (a retry of an LLM call with `rephrase: TRUE` doubles cost), but there is no ceiling.

**Recommendation:** Add a `BUDGET` scope primitive (analogous to `SANDBOX`) that constrains cumulative LLM token usage, API call count, and wall-clock time across a pipeline or delegation tree.

### 1.2 No Prompt Management or Template System

**Severity: High**

The `agent.llm` module treats prompts as opaque `Str` values. For an AI-native language, prompts are the most critical artifact — they are the "source code" that agents feed to LLMs. Yet:

- No typed prompt templates with slot validation.
- No prompt versioning or content-addressing of prompt artifacts.
- No prompt composition operators (system/user/assistant message structure).
- No few-shot example injection pattern.
- `core.text.FORMAT` exists but is a generic string template — it has no awareness of LLM message structure (roles, tool-use schemas, etc.).

**Recommendation:** Add a `PROMPT` schema type that models multi-turn message sequences with typed slots, version tracking, and composition operators. This is arguably more important than `core.math.TRIG` for an AI-native language.

### 1.3 No Observability Into Probabilistic Reasoning Chains

**Severity: High**

The probabilistic type system enforces confidence thresholds at elimination sites, but there is no mechanism for an agent (or human auditor) to inspect *why* a confidence value is what it is:

- Provenance is a `Hash` reference, but the format of the provenance object is never normatively defined (noted in `formal_semantics.md` §14.2).
- There is no "explain confidence" operation that decomposes a confidence score into its contributing factors.
- The composition laws (§6.2 of formal semantics: `c_seq = c1 * c2`, `c_retry = 1-(1-c)^n`) are presented but there is no mechanism to trace which law was applied at each step.
- An agent receiving a `Probable[T]` with `confidence: ~0.73` has no way to ask "what would increase this confidence?"

**Recommendation:** Define a normative `ProvenanceChain` schema that records confidence composition steps. Add a `EXPLAIN_CONFIDENCE` stdlib operation that decomposes a `Probable[T]` into its contributing factors.

### 1.4 No Learning / Adaptation Primitive

**Severity: Medium**

The spec includes `agent.reflection` with `EVALUATE`, `CRITIQUE`, `IMPROVE`, and `LEARN` — but the stdlib only specifies `EVALUATE` and `CRITIQUE`. The `IMPROVE` and `LEARN` operations are listed in the base spec's §8.2 table but have no stdlib contracts. This is a significant gap for an AI-native language:

- How does an agent update its own behavior based on feedback?
- `SELF_MODIFY` capability exists, and §7.4 describes the verification protocol for self-modification, but there is no stdlib operation that bridges reflection results to self-modification proposals.
- The `LEARN` operation is exactly the kind of primitive that differentiates an AI-native language from a general-purpose one.

**Recommendation:** Specify `IMPROVE` and `LEARN` in the stdlib. Define how reflection outputs feed into verified self-modification proposals. This completes the "cognition loop" that makes the language truly agent-native.

### 1.5 No Multi-Modal Input/Output

**Severity: Medium**

AgentLang's type system has no representation for images, audio, video, or other non-text modalities. Modern LLMs are multi-modal. The `agent.llm.GENERATE` operation accepts `prompt: Str` — there is no way to pass an image for vision tasks, audio for transcription, or structured tool-use results.

**Recommendation:** Add `Blob[mime_type]` or `Media` composite type. Extend `agent.llm` operations to accept multi-modal inputs.

### 1.6 No Agent Discovery or Marketplace Protocol

**Severity: Low-Medium**

The base spec mentions an "Agent Registry" in the runtime (§12.2) but there is no language-level mechanism for:

- Discovering agents by capability at runtime.
- Negotiating trust levels before delegation.
- Advertising available operations to other agents.
- A "marketplace" pattern where agents can offer and consume services dynamically.

The `DELEGATE` keyword requires naming a specific agent. There is no `DISCOVER` + `DELEGATE` pattern.

**Recommendation:** Add `DISCOVER` as a coordination keyword that queries the agent registry with capability/trust filters and returns candidate agents for delegation.

---

## 2. Cross-Document Contradictions

### 2.1 FLATTEN Type Signature Contradicts Type Safety Claims

**Base spec §2.2.1:** `FLATTEN` — "Reduce nesting depth by one level."
**stdlib_spec §2.1 (core.data.FLATTEN):** `FLATTEN[T](items: List[Any], depth: UInt8=1) -> List[Any]`

The use of `Any` in both input and output types is inconsistent with AgentLang's strong static typing philosophy. The formal semantics defines a type system where every expression has a precise type — `List[Any]` is an escape hatch that undermines the type checker. A well-typed FLATTEN should be:
- `FLATTEN[T](items: List[List[T]], depth: 1) -> List[T]`
- Or use a recursive type-level depth computation.

### 2.2 REDUCE Signature Inconsistency Between Spec and Stdlib

**Base spec §2.2.1:** `REDUCE` — "Aggregate elements to single value."
**Base spec §4.1 example:** `SOURCE sales -> REDUCE SUM revenue` — uses keyword-style shorthand.
**stdlib_spec:** `REDUCE[T,A](items: List[T], seed: A, reducer: (A,T) -> A) -> A` — uses explicit seed and reducer function.

The pipeline syntax `REDUCE SUM revenue` has no corresponding formal representation. Is `SUM` a built-in reducer? Is `revenue` a field accessor? The stdlib defines REDUCE as a general fold, but the pipeline examples use a completely different invocation style. The EBNF grammar does not account for this shorthand form.

### 2.3 TOKENIZE Return Type is `List[Any]`

**stdlib_spec §2.4 (core.text.TOKENIZE):** Returns `List[Any]`.

For a language that mandates explicit type handling, returning `Any` from a tokenizer is unacceptable. Different strategies produce different types:
- `whitespace` → `List[Str]`
- `bpe` → `List[UInt32]` (token IDs)
- `wordpiece` → `List[Str]`
- `sentence` → `List[Str]`

This should be a type-level dispatch or return a union type.

### 2.4 REGEX Return Type is `Any`

**stdlib_spec §2.4 (core.text.REGEX):** Returns `Any`.

The return type depends on the operation:
- `match` → `Bool`
- `search` → `List[MatchResult]` or `NONE`
- `replace` → `Str`
- `split` → `List[Str]`

Returning `Any` again contradicts the type safety guarantees. This should be split into separate operations or use dependent typing on the `op` parameter.

### 2.5 Capability Naming Inconsistency

**Base spec §6.2:** Defines capabilities like `DB_READ`, `DB_WRITE`, `API_CALL`, `FILE_READ`, `FILE_WRITE`.
**stdlib_spec §2.2 (core.io.READ):** Requires "read capability."
**stdlib_spec §2.2 (core.io.WRITE):** Requires "write capability."
**stdlib_spec §3.1 (agent.llm):** Requires "LLM capability."
**stdlib_spec §3.3 (agent.tools):** Requires "register capability," "invoke capability."

The stdlib introduces new capability names (`LLM`, `register`, `invoke`, `reflect`, `scheduler`) that are not defined in the base spec's capability table (§6.2). The formal semantics defines `Cap` as a set but does not enumerate members. There is no canonical capability registry.

### 2.6 EMIT Semantics — Keyword vs. Channel Send

**Base spec §2.2.2:** `EMIT` — "Produce output value. Replaces return; supports streaming."
**Base spec §6.4:** `EMIT TO task_updates => TaskUpdate {...}` — used as a channel send operation.
**Formal semantics:** `EMIT` is not formalized as a core calculus term.

`EMIT` is overloaded: it serves as both a "return value from operation/pipeline" and a "send message to channel" mechanism. These are semantically very different operations. The formal semantics has no reduction rule for `EMIT`, which means the most fundamental output mechanism of the language is unformalized.

### 2.7 Pipeline Operator Ambiguity: `->` vs `|>`

**Base spec §2.3:** Defines both `->` (Pipeline/Transform) and `|>` (Pipe with context).
**Formal semantics §4.1 (E-Pipe):** Only formalizes `->` as `v -> op ≡ op(v)`.
**stdlib_spec:** All examples use `->`.

The `|>` operator is introduced but never used in any example, never formalized, and its "context" parameter is undefined. What is `ctx` in `data |> TRANSFORM(ctx)`? How does it differ from `data -> TRANSFORM(ctx)`? This operator should either be removed or properly specified.

### 2.8 Delegation Capability Model

**Formal semantics §14.5:** "Whether callee runs with caller caps, own caps, or intersection should be explicit."
**Base spec §6.3:** Delegation includes `ISOLATION` block but describes data access policies, not capability policies.

This is flagged by the formal semantics itself as an open question. When Agent A (with `DB_WRITE`) delegates to Agent B (with `DB_READ` only), and the delegated operation requires `DB_WRITE`:
- Does the operation run with A's caps? (capability escalation risk)
- Does it run with B's caps? (task will fail)
- Does it run with the intersection? (always fails for heterogeneous delegation)

### 2.9 STORE vs. Mutable: Formal vs. Surface Syntax

**Base spec §5.2:** `STORE customers = LOAD("db://main/customers")` — `STORE` creates a named immutable reference.
**Base spec §5.3:** `MUTABLE cursor @reason("...")` — separate keyword for mutable bindings.
**Formal semantics §4.1:** `store x = e` and `mutable x = e | assign x = e` — separate reduction rules.

The formal semantics correctly separates these, but the base spec's EBNF grammar (§15) does not include production rules for `STORE` or `MUTABLE` declarations. They are mentioned in the keyword table but absent from the grammar.

### 2.10 Missing Stdlib Operations

**Base spec §8.1-8.3** lists these operations that have no stdlib contracts:

| Module | Missing Operations |
|---|---|
| `core.data` | `PIVOT`, `WINDOW`, `ACCUMULATE` |
| `core.crypto` | `DECRYPT` |
| `core.text` | (none — all covered) |
| `agent.llm` | `SUMMARIZE` |
| `agent.memory` | `SEARCH` |
| `agent.tools` | `DISCOVER` |
| `agent.planning` | `PRIORITIZE`, `REPLAN` |
| `agent.reflection` | `IMPROVE`, `LEARN` |
| `db.sql` | `DELETE`, `MIGRATE` |
| `db.graph` | `TRAVERSE`, `SHORTEST_PATH`, `PATTERN_MATCH` (entire module) |
| `api.grpc` | `SERVICE`, `METHOD`, `STREAM_IN`, `STREAM_OUT` (entire module) |
| `queue.pubsub` | `ACK`, `NACK`, `REPLAY` |

The stdlib claims "58 operations specified" but the base spec promises significantly more. The entire `db.graph` and `api.grpc` modules are missing.

### 2.11 FAILURE Type vs. Result Type

**Base spec §4.6:** Uses `Result` type with `SUCCESS(T) | FAILURE(ErrorCode, Str)` — two-field failure.
**stdlib_spec §1.2:** Uses `FAILURE(code, message, details)` — three-field failure.

The Result union type defined in §3.6 of the base spec has `FAILURE(ErrorCode, Str)` (two parameters), but every stdlib operation documents failure as `FAILURE(code, message, details)` (three parameters). The formal semantics uses `failure(ε, m)` (two parameters). These must be reconciled.

### 2.12 `HttpResponse[T]` Type Not Defined

**stdlib_spec §2.8:** `core.http.GET` returns `HttpResponse[T]`, but this type is never defined in any document. There is no schema for it. What fields does it contain? Status code? Headers? Is the body always decoded, or can it fail partially?

---

## 3. Edge Cases in the Execution Model

### 3.1 Dataflow DAG

#### 3.1.1 Dynamic DAG Construction

The formal semantics models the DAG as a static structure `G=(V,E)`, but the base spec's examples show DAGs that are constructed dynamically based on data:

```
-> MAP order =>
    history = QUERY db://customers WHERE id EQ order.customer_id
    DELEGATE assess_fraud TO fraud_detector
        INPUT {order, customer_history: history}
```

Here, each element in the MAP creates a new subgraph (QUERY + DELEGATE). The DAG shape depends on the number of orders — it is not statically known. The formal semantics does not address:
- How dynamic DAG expansion interacts with the scheduler.
- Whether there is a limit on DAG node count.
- How checkpointing works when the DAG shape is data-dependent.

#### 3.1.2 DAG Cycles Through Mutable State

The spec states programs are "directed acyclic graphs." But consider:

```
MUTABLE counter @reason("tracking")
OBSERVE some_channel =>
    WHEN event_received -> counter = counter + 1
```

The `OBSERVE` keyword creates a reactive subscription that can fire indefinitely. Combined with mutable state, this creates effective cycles. The formal semantics does not model `OBSERVE` at all — it has no reduction rule.

#### 3.1.3 FORK Branch Failure Semantics Under BEST_EFFORT

**Formal semantics §10.2:** Defines join strategies but leaves precise behavior undefined.

If `FORK` has 3 branches and strategy is `BEST_EFFORT`:
- What value does a failed/timed-out branch contribute to the merge? `NONE`? `FAILURE`?
- If all branches fail, does the join proceed with empty results or does the pipeline fail?
- What happens to side effects (e.g., `DB_WRITE`) in branches that completed before the timeout?
- Is there a compensation/rollback mechanism for partial completion?

#### 3.1.4 Pipeline Stage Type Inference Across Dynamic Branches

In the complete example (§14):
```
-> FORK {
    approved: FILTER recommendation EQ approve -> MAP finalize_order,
    flagged:  FILTER recommendation EQ review  -> MAP flag_for_review -> ESCALATE TO human_reviewer,
    rejected: FILTER recommendation EQ reject  -> MAP cancel_order -> notify_customer
}
-> EMIT summary
```

Each branch produces a different type. What is the type of the merged output? How is `summary` typed? The formal semantics does not define typing rules for `FORK/JOIN` — only operational semantics.

### 3.2 Checkpointing

#### 3.2.1 Checkpoint Consistency With Shared Mutable State

**Formal semantics §14.9:** "Checkpoint consistency model (snapshot isolation vs. eventual consistency for shared scope) is not defined."

If Agent A checkpoints while Agent B is mid-write to `SHARED` state:
- Does the checkpoint capture A's view (potentially stale)?
- Is there a read barrier?
- Can checkpoint restore put the system in a state that never existed (A's local state + B's partial shared state)?

#### 3.2.2 Checkpoint Across Delegation Boundaries

When a pipeline checkpoints after delegating to another agent:
```
-> DELEGATE validate TO processor -> CHECKPOINT -> DELEGATE ship TO shipper
```

What does the checkpoint capture?
- Only the local agent's state? (Resume may re-delegate validation unnecessarily.)
- The delegated agent's completion result? (How, if the delegate has its own lifecycle?)
- The entire delegation tree's state? (Distributed snapshot problem.)

#### 3.2.3 Checkpoint + MUTABLE Interaction

The CAS memory model stores immutable values by hash. But `MUTABLE` cells exist in a separate mutable map `M`. When a checkpoint is taken:
- Are mutable cell values serialized into the checkpoint?
- If so, restoring creates a snapshot of mutable state — but other agents may have moved forward. Is there a conflict detection mechanism?
- The formal semantics checkpoint rule (`E-Checkpoint`) snapshots `(ρ, Σ, Q)` which includes `M`, but does not address multi-agent consistency.

#### 3.2.4 State Migration Completeness

`MIGRATE STATE v1 -> v2` supports `ADD_FIELD`, `RENAME_FIELD`, `DROP_FIELD`, `TRANSFORM`. But:
- What about type changes (e.g., `Int32` to `Int64`)?
- What about schema restructuring (e.g., flattening a nested schema)?
- Is migration reversible? Can you migrate v2 -> v1?
- What happens to in-flight checkpoints during a migration? Are they automatically migrated or invalidated?

### 3.3 Probabilistic Types

#### 3.3.1 Confidence Composition Independence Assumption

**Formal semantics §6.2:** Sequential confidence is `c_seq = c1 * c2`, assuming independence.

In practice, LLM calls in a pipeline are almost never independent — the output of one call is the input to the next. If `GENERATE` produces a summary with `c=0.9`, and `CLASSIFY` classifies that summary with `c=0.85`, the real joint confidence is not `0.765` — it depends on the conditional probability `P(classify_correct | summary_correct)`. The spec acknowledges "tighter estimators" are allowed but provides no mechanism for agents to declare or exploit dependency structure.

#### 3.3.2 Confidence Calibration Is Undefined

Where do confidence values come from? The spec assumes LLM operations produce calibrated confidence scores, but:
- LLM log-probabilities are notoriously uncalibrated.
- There is no calibration protocol or reference.
- An agent could assign `confidence: ~0.99` to arbitrary outputs.
- The formal semantics treats confidence as an opaque `[0,1]` value with no semantic grounding.

**This is arguably the deepest problem in the spec.** The entire probabilistic type system's value proposition depends on confidence values being meaningful, but nothing in the specification ensures they are.

#### 3.3.3 Degradation Policy Execution

A `Probable[T]` carries a degradation policy (e.g., `RETRY(3) -> ESCALATE`). But:
- Who executes the degradation policy? The operation that produced the value? The consumer?
- When is it triggered? Only at `use` sites? At any confidence check?
- Can degradation policies themselves fail? What happens if `ESCALATE` has no handler?
- The formal semantics includes `d` in the tuple but has no reduction rule for degradation policy execution.

#### 3.3.4 Probable[Probable[T]] — Nested Uncertainty

Nothing in the spec prevents `Probable[Probable[T]]` — an uncertain value whose inner value is also uncertain. This could arise naturally:
```
result = DELEGATE classify TO untrusted_agent  // Probable[Probable[ENUM(...)]]
```

The delegation attenuates confidence (`c_eff = c * t_agent`), wrapping an already-Probable output. The elimination rules would require nested `use` expressions. Is this intentional? Should there be a flattening rule?

#### 3.3.5 Confidence on Non-LLM Operations

The FFI example assigns `confidence: ~0.99` to a deterministic Python computation. What does confidence mean for deterministic operations? Is it:
- Probability of correct execution? (Should be 1.0 for deterministic code.)
- Trust in the FFI boundary? (Then it should be a property of the FFI bridge, not the value.)
- A formality to satisfy the type system? (Then it's meaningless noise.)

### 3.4 Other Execution Model Edge Cases

#### 3.4.1 Bounded Loop HALT Semantics

```
LOOP max: 100 =>
    page = fetch_next_page(cursor)
    WHEN page.empty -> HALT(COMPLETE)
    process_page(page)
```

What happens when the loop hits `max: 100` without reaching `HALT(COMPLETE)`?
- Is it a runtime error?
- Does it silently complete with whatever was processed?
- Does it trigger the agent's `ON_FAILURE` policy?
- The formal semantics desugars to `iter(k, s0, f, halt_pred)` but does not define the behavior when fuel is exhausted without the halt predicate being satisfied.

#### 3.4.2 MATCH Exhaustiveness With Dependent/Refinement Types

```
discount: Float64 :: range(0.0..0.5)
MATCH discount =>
    WHEN GT 0.3 -> ...
    WHEN GT 0.1 -> ...
    // Is this exhaustive? The compiler would need to prove that
    // range(0.0..0.5) is covered by (>0.3) ∪ (>0.1) ∪ (≤0.1 ∩ ≥0.0)
```

The exhaustiveness checker must integrate with the SMT solver to verify coverage over continuous ranges. This is significantly more complex than enum exhaustiveness and is not addressed.

#### 3.4.3 Concurrent ACQUIRE With Timeout Expiry

```
ACQUIRE lock ON shared_counter TIMEOUT 5s =>
    shared_counter = shared_counter + batch_size
```

If the timeout expires while waiting:
- Is it a `FAILURE(TIMEOUT, ...)`?
- Does it trigger the agent's `ON_FAILURE` policy?
- Is the calling pipeline interrupted or does it continue without the lock?
- The formal semantics has `E-AcquireBusy` which transitions to `blocked(r,e)` but has no rule for timeout-triggered unblocking.

#### 3.4.4 Bidirectional Sync Operator `<=>`

The operator `<=>` is listed in §2.3 as "Bidirectional sync: `local_state <=> remote_state`" but:
- It has no formal semantics.
- It has no EBNF grammar production.
- It has no stdlib operation.
- It implies continuous synchronization, which is fundamentally at odds with the dataflow DAG model.
- What conflict resolution strategy does it use? Last-write-wins? CRDTs?

---

## 4. Ergonomics — Is This Syntax Easy for an LLM to Generate/Parse?

### 4.1 Inconsistent Invocation Styles

The spec mixes at least four different invocation patterns:

| Pattern | Example | Context |
|---|---|---|
| Pipeline keyword | `-> FILTER status EQ active` | Data operations in pipelines |
| Function call | `FILTER(items, predicate)` | Stdlib formal signatures |
| Method-style | `LLM.CLASSIFY { model: ..., input: ... }` | Agent module calls |
| Block-style | `DELEGATE x TO y => INPUT ... TIMEOUT ...` | Agent coordination |

An LLM generating AgentLang code must learn four distinct syntactic patterns for what are conceptually the same thing: calling an operation with arguments. This is a significant source of generation errors. The pipeline style `FILTER status EQ active` is particularly ambiguous — is `status` a field name, a variable, or the first argument? Is `EQ` an operator or a keyword?

**Recommendation:** Unify invocation syntax. Choose one pattern (pipeline keyword + named arguments is the most readable) and use it consistently.

### 4.2 Keyword Overload and Ambiguity

Several keywords serve multiple roles:

- `=>` means "produces" in schemas (`SCHEMA User => {...}`), "body follows" in operations (`OPERATION x => ...`), and "maps to" in type aliases (`TYPE Result[T] = SUCCESS(T) | FAILURE(...)`). An LLM must use context to disambiguate.
- `->` means "pipeline stage" (`data -> FILTER`) and "then do" in failure policies (`RETRY(3) -> ESCALATE`).
- `WHEN` is used in `MATCH` blocks, standalone conditionals, and within `JOIN` strategies.

### 4.3 Whitespace Sensitivity Is Undefined

The spec uses indentation in all examples but never states whether whitespace is significant. Is this:

```
OPERATION x =>
    INPUT a: Int
    BODY {
        EMIT a + 1
    }
```

...different from this?

```
OPERATION x => INPUT a: Int BODY { EMIT a + 1 }
```

The EBNF grammar uses `{` `}` delimiters suggesting whitespace-insensitivity, but the examples rely heavily on indentation for readability. For LLM generation, explicit delimiters (braces/semicolons) are far more reliable than significant whitespace. This should be explicitly stated.

### 4.4 Semicolons, Commas, and Statement Termination

The spec never defines statement termination. Are statements separated by newlines? Semicolons? Nothing?

- FORK branches use commas: `branch_a: ..., branch_b: ..., branch_c: ...`
- SCHEMA fields use commas (implied by examples).
- Pipeline stages use `->`.
- Top-level declarations use... nothing visible.

An LLM needs unambiguous rules for where one statement ends and another begins. The current spec leaves this entirely to intuition from examples.

### 4.5 Naming Convention Inconsistency

- Keywords: `UPPER_SNAKE` (`FILTER`, `MATCH`, `CHECKPOINT`)
- Types: `PascalCase` (`Float64`, `Probable`, `List`)
- Variables: `snake_case` (`customer_id`, `batch_data`)
- Schema fields: `snake_case` (`risk_score`, `order_id`)
- Capabilities: `UPPER_SNAKE` (`DB_READ`, `API_CALL`)
- Enum values: `lowercase` in some places (`active`, `pending`), `UPPER_SNAKE` in others (`COMPLETED`, `NOT_FOUND`)

The enum value inconsistency is particularly problematic. In the same example program, `status: ENUM(pending, verified, shipped, delivered, cancelled)` uses lowercase, while `FAILURE(NOT_FOUND, msg)` uses uppercase. An LLM will frequently mix these up.

**Recommendation:** Establish a single, documented naming convention. Enums should consistently be either `lowercase` or `UPPER_CASE`.

### 4.6 The `::` Constraint Operator Is Overloaded With Type Syntax

In many languages, `::` is used for type annotations or namespacing. In AgentLang, `:` is the type annotation and `::` is the constraint annotation:

```
age: UInt8 :: range(0..150)
latency :: <50ms
```

The second form (`latency :: <50ms`) has no type annotation — only a constraint. This is syntactically novel but potentially confusing for LLMs trained on languages where `::` means "has type." It also creates ambiguity: is `x :: range(0..10)` a constraint on an existing binding `x`, or a declaration of a new binding with only a constraint and no type?

### 4.7 Lack of Explicit Module Import Syntax

The stdlib defines modules (`core.data`, `agent.llm`, etc.) but there is no `IMPORT` or `USE` statement in the language. The base spec's EBNF grammar has no import production. Examples use operations like `LLM.CLASSIFY` and `SQL.QUERY` with module prefixes but never show how modules are brought into scope.

For LLM code generation, explicit imports are valuable because they:
- Provide a clear "header" that establishes available operations.
- Reduce ambiguity about which module an operation belongs to.
- Enable the LLM to scope its generation to relevant APIs.

### 4.8 No Comment Syntax Defined

The spec uses `//` for comments in examples, but the EBNF grammar has no comment production. Are block comments supported? Is `/* ... */` valid? For LLM-generated code, comments are less important than for human code, but they are valuable for:
- Verification annotations and proof hints.
- Explaining degradation policy choices.
- Documenting prompt engineering rationale.

### 4.9 Error Recovery and Partial Parsing

If an LLM generates syntactically invalid AgentLang, the spec provides no guidance on:
- Error recovery strategies for the parser.
- Whether partial programs can be type-checked (e.g., checking one operation even if another has a syntax error).
- Incremental parsing support for streaming code generation.

For an AI-native language, the compiler should be designed to provide maximal useful feedback on malformed programs, since the "developer" (an LLM) will use that feedback to iterate.

---

## 5. Additional Observations

### 5.1 The `ASSUME` Keyword Is a Soundness Escape Hatch

`ASSUME` declares "unverified premises" — this is essentially a way to introduce axioms that the verification system trusts without proof. In a language designed for AI agents that "may hallucinate," this is dangerous. An agent could `ASSUME` away all verification obligations. The spec should define:
- Limits on what can be assumed.
- Mandatory audit trail for all `ASSUME` usage.
- Whether `ASSUME`-dependent code paths are marked as "unverified" in the type system.

### 5.2 The Trust Level Model Is Static

Agent trust levels are declared as constants (`TRUST_LEVEL ~0.92`). In practice, trust should be dynamic:
- An agent that has been producing correct results should gain trust.
- An agent that has failed should lose trust.
- Trust should decay over time without evidence of continued reliability.

The spec has no mechanism for trust level updates, though `SELF_MODIFY` could theoretically be used to update one's own trust level (which is a security concern).

### 5.3 No Notion of "Context Window" or "Working Memory"

LLM-backed agents have finite context windows. AgentLang has `AGENT` memory scoping and `agent.memory` operations, but there is no concept of:
- How much context an agent can hold simultaneously.
- When to page information out of "working memory" into persistent storage.
- How pipeline state interacts with an LLM's context window during execution.

This is critical for practical implementation — an agent executing a complex pipeline cannot hold the entire pipeline state in its LLM context.

### 5.4 Formal Semantics Lacks STREAM/OBSERVE/BROADCAST

The formal semantics defines reduction rules for core constructs but entirely omits:
- `STREAM` — lazy evaluation with backpressure.
- `OBSERVE` — reactive subscriptions.
- `BROADCAST` — pub/sub messaging.
- `EMIT` — the fundamental output mechanism.

These are all listed as keywords in the base spec but have no formal model. For a "normative draft for production runtime/compiler implementations," this is a significant gap.

### 5.5 No Formal Grammar for Constraint Expressions

Constraints like `:: range(0..150)`, `:: length(1..200)`, `:: pattern(EMAIL_REGEX)` are used throughout but:
- The EBNF grammar defines `constraint = identifier "(" arg_list ")"` which is extremely vague.
- There is no enumeration of built-in constraint functions.
- There is no way to define custom constraints.
- The formal semantics represents constraints as refinement predicates `{x:τ | φ}` but does not define how surface constraint syntax maps to the refinement language `φ`.

---

## 6. Summary of Recommendations (Priority-Ordered)

| Priority | Recommendation | Section |
|---|---|---|
| P0 | Define confidence calibration protocol and provenance format | 3.3.2, 1.3 |
| P0 | Reconcile invocation syntax (pipeline vs. function vs. block) | 4.1 |
| P0 | Formalize EMIT, STREAM, OBSERVE semantics | 2.6, 5.4 |
| P0 | Define FORK/JOIN failure and typing semantics precisely | 3.1.3, 3.1.4 |
| P0 | Reconcile FAILURE type arity (2 vs. 3 params) | 2.11 |
| P1 | Add token budget / cost accounting primitive | 1.1 |
| P1 | Add typed prompt template system | 1.2 |
| P1 | Define delegation capability model (caller vs. callee caps) | 2.8 |
| P1 | Define checkpoint consistency model for shared state | 3.2.1 |
| P1 | Specify all missing stdlib operations (especially db.graph, api.grpc) | 2.10 |
| P1 | Define loop fuel exhaustion semantics | 3.4.1 |
| P1 | Define statement termination and whitespace rules | 4.3, 4.4 |
| P2 | Fix FLATTEN/TOKENIZE/REGEX to use precise types instead of Any | 2.1, 2.3, 2.4 |
| P2 | Unify enum value naming convention | 4.5 |
| P2 | Add DISCOVER keyword for dynamic agent lookup | 1.6 |
| P2 | Define or remove `<=>` bidirectional sync operator | 3.4.4 |
| P2 | Define or remove `|>` pipe-with-context operator | 2.7 |
| P2 | Specify IMPROVE and LEARN stdlib operations | 1.4 |
| P2 | Add multi-modal type support | 1.5 |
| P3 | Restrict or audit ASSUME usage | 5.1 |
| P3 | Add dynamic trust level updates | 5.2 |
| P3 | Define comment syntax in grammar | 4.8 |
| P3 | Add module import syntax | 4.7 |

---

*End of Review*
