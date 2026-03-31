# Phase 6: React, Don't Poll — Reactive Channels

**Principle:** Agent-Native Coordination (reactive extension)
**Status:** Planned
**Depends on:** Phase 3 (concurrent runtime), Phase 4 (real backends for I/O)

---

## 1. Overview

The MVP is entirely imperative. This phase adds event-driven multi-agent communication through typed channels, reactive observation, broadcast messaging, and streaming. Agents coordinate reactively — subscribing to events rather than polling for state changes.

## 2. Requirements

### 2.1 Channel Type and Runtime

- **R1.1:** `CHANNEL name: Queue[T] => { MAX_SIZE, OVERFLOW_POLICY, SUBSCRIBERS }` is a new declaration.
- **R1.2:** `Queue[T]` is a typed, bounded channel carrying values of type `T`.
- **R1.3:** `MAX_SIZE` — channel buffer size (e.g., `1000`).
- **R1.4:** `OVERFLOW_POLICY` — `DROP_OLDEST`, `BLOCK`, `ERROR` when channel is full.
- **R1.5:** `SUBSCRIBERS` — list of agents permitted to read from the channel.
- **R1.6:** `EMIT value TO channel` sends a value to a channel. Requires `QUEUE_PUBLISH` capability.
- **R1.7:** Channel messages are schema-validated at send time (type error if value doesn't match `T`).

### 2.2 OBSERVE Reactive Subscriptions

- **R2.1:** `OBSERVE channel => { WHEN condition -> action }` subscribes to channel events.
- **R2.2:** Requires `QUEUE_SUBSCRIBE` capability.
- **R2.3:** Multiple `WHEN` handlers per observation.
- **R2.4:** Handlers trigger asynchronously when a matching message arrives.
- **R2.5:** Subscription lifecycle: created on OBSERVE, destroyed on agent termination or explicit unsubscribe.
- **R2.6:** Cycle detection: compiler warns if OBSERVE handlers can trigger themselves through mutable state.

### 2.3 BROADCAST

- **R3.1:** `BROADCAST value` sends to all agents in scope.
- **R3.2:** Per-sender FIFO ordering guarantee.
- **R3.3:** Subscriber failure isolation: one subscriber's failure doesn't affect others.
- **R3.4:** Requires `QUEUE_PUBLISH` capability.

### 2.4 STREAM Operator

- **R4.1:** `STREAM` represents an unbounded sequence of values.
- **R4.2:** Lazy evaluation: values produced on demand.
- **R4.3:** Backpressure: slow consumers cause producers to pause.
- **R4.4:** Cancellation: streams can be terminated by consumer.
- **R4.5:** Pipeline integration: `STREAM source |> FILTER |> MAP |> TAKE(10)` works with existing pipeline syntax.

## 3. Architecture

### 3.1 Crate Changes

**New crate: `al-channels`**
- Channel type definitions and runtime
- Message queue implementation (bounded, typed)
- Subscription management
- Broadcast dispatcher
- Stream implementation with backpressure

**`al-ast`:**
- New `Declaration::Channel` variant
- New `Statement::EmitTo { value, channel }` variant
- New `Statement::Observe { channel, handlers }` variant
- New `Statement::Broadcast { value }` variant
- New `TypeExpr::Stream(Box<TypeExpr>)` variant

**`al-lexer` / `al-parser`:**
- New keywords: `CHANNEL`, `OBSERVE`, `BROADCAST`, `STREAM`, `TO`, `SUBSCRIBERS`, `MAX_SIZE`, `OVERFLOW_POLICY`

**`al-types`:**
- Channel type checking: `Queue[T]` type validation
- EMIT TO type checking: value must match channel's `T`
- OBSERVE handler type checking
- Stream type propagation through pipelines

**`al-runtime`:**
- Channel runtime: concurrent message passing
- Observation dispatcher: async handler invocation
- Broadcast implementation
- Stream evaluation engine

## 4. Testing

### 4.1 Unit Tests — Channels (`al-channels`)

| ID | Test | Description |
|----|------|-------------|
| T1.1 | `test_channel_send_receive` | Send value to channel, receive it |
| T1.2 | `test_channel_typed` | Channel rejects value of wrong type |
| T1.3 | `test_channel_bounded` | Channel with MAX_SIZE 3: 4th send triggers overflow policy |
| T1.4 | `test_channel_drop_oldest` | OVERFLOW_POLICY DROP_OLDEST discards oldest message |
| T1.5 | `test_channel_block` | OVERFLOW_POLICY BLOCK pauses sender until space available |
| T1.6 | `test_channel_error` | OVERFLOW_POLICY ERROR returns FAILURE on full channel |
| T1.7 | `test_channel_subscribers` | Only listed subscribers can read |
| T1.8 | `test_channel_fifo` | Messages received in FIFO order |

### 4.2 Unit Tests — OBSERVE (`al-channels`)

| ID | Test | Description |
|----|------|-------------|
| T2.1 | `test_observe_handler_triggers` | OBSERVE handler fires when matching message arrives |
| T2.2 | `test_observe_condition_filter` | WHEN condition filters messages correctly |
| T2.3 | `test_observe_multiple_handlers` | Multiple WHEN handlers on same channel |
| T2.4 | `test_observe_unsubscribe` | Agent termination cleans up subscription |
| T2.5 | `test_observe_requires_capability` | OBSERVE without QUEUE_SUBSCRIBE → CapabilityDenied |
| T2.6 | `test_observe_cycle_warning` | Handler that modifies observed state → compiler warning |

### 4.3 Unit Tests — BROADCAST (`al-channels`)

| ID | Test | Description |
|----|------|-------------|
| T3.1 | `test_broadcast_all_agents` | BROADCAST delivers to all agents in scope |
| T3.2 | `test_broadcast_fifo_per_sender` | Messages from same sender arrive in order |
| T3.3 | `test_broadcast_subscriber_isolation` | One subscriber failure doesn't affect others |
| T3.4 | `test_broadcast_requires_capability` | BROADCAST without QUEUE_PUBLISH → CapabilityDenied |

### 4.4 Unit Tests — STREAM (`al-channels`)

| ID | Test | Description |
|----|------|-------------|
| T4.1 | `test_stream_lazy` | Stream values not produced until consumed |
| T4.2 | `test_stream_backpressure` | Slow consumer pauses fast producer |
| T4.3 | `test_stream_cancellation` | Consumer cancelling stream stops producer |
| T4.4 | `test_stream_pipeline` | `STREAM source |> FILTER |> MAP |> TAKE(5)` returns 5 values |
| T4.5 | `test_stream_take` | TAKE(n) terminates stream after n values |

### 4.5 Unit Tests — Parser (`al-parser`)

| ID | Test | Description |
|----|------|-------------|
| T5.1 | `test_parse_channel_decl` | CHANNEL declaration parses correctly |
| T5.2 | `test_parse_emit_to` | `EMIT value TO channel` parses correctly |
| T5.3 | `test_parse_observe` | OBSERVE with WHEN handlers parses correctly |
| T5.4 | `test_parse_broadcast` | BROADCAST statement parses correctly |
| T5.5 | `test_parse_stream_type` | `Stream[Int64]` type expression parses correctly |

### 4.6 Integration Tests

| ID | Test | Description |
|----|------|-------------|
| T6.1 | `test_channel_e2e` | Two agents communicate through typed channel |
| T6.2 | `test_observe_react_e2e` | Agent reacts to channel event automatically |
| T6.3 | `test_broadcast_e2e` | Agent broadcasts to all, multiple agents receive |
| T6.4 | `test_stream_pipeline_e2e` | Stream through pipeline produces expected results |

### 4.7 Conformance Tests

| ID | Test | Description |
|----|------|-------------|
| C41 | `conformance_typed_channels` | Channels enforce type safety |
| C42 | `conformance_observe_reactive` | OBSERVE triggers handlers on message arrival |
| C43 | `conformance_broadcast_delivery` | BROADCAST delivers to all agents in scope |
| C44 | `conformance_stream_backpressure` | Stream backpressure prevents producer overrun |

## 5. Acceptance Criteria

- [ ] CHANNEL is a new declaration type with typed, bounded message queues
- [ ] EMIT TO sends typed messages to channels with capability gating
- [ ] OBSERVE creates reactive subscriptions that trigger on message arrival
- [ ] BROADCAST delivers to all agents with per-sender FIFO ordering
- [ ] STREAM provides lazy, backpressure-aware sequences
- [ ] Cycle detection warns on potential infinite reactive loops
- [ ] All existing tests pass unchanged
- [ ] 4 new conformance tests (C41-C44) pass
