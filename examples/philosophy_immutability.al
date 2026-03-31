// ============================================================
// AgentLang Philosophy: Immutable by Default
// ============================================================
//
// STORE bindings are immutable. Once set, they cannot change.
// This prevents accidental side effects and makes reasoning simple.
//
// MUTABLE requires explicit justification with @reason.
// The reason is not a comment — it is a language-level annotation
// that documents WHY this state needs to change.
//
// In a world of autonomous agents, mutable state is a liability.
// Every mutation must be justified, tracked, and auditable.
//
// Expected result: 918

OPERATION process =>
  BODY {
    // Immutable bindings — cannot be changed after creation
    STORE initial_value = 10
    STORE multiplier = 3
    STORE offset = 5

    // Mutable state requires explicit reason
    MUTABLE accumulator @reason("building result through stages") = 0
    MUTABLE stage @reason("tracking computation phase") = 1

    // Stage 1: multiply
    accumulator = initial_value * multiplier
    stage = stage + 1

    // Stage 2: square
    accumulator = accumulator * accumulator
    stage = stage + 1

    // Stage 3: take remainder to keep manageable
    accumulator = accumulator - (accumulator / 1000) * 1000
    stage = stage + 1

    // The immutable values are still exactly what they were
    STORE check = initial_value + multiplier + offset
    EMIT check + accumulator
  }

PIPELINE Main => process
