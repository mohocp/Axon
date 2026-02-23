# AgentLang Standard Library MVP Specification (v0.1)

Status: Normative standard-library surface for MVP compiler/runtime conformance.

This document contains only operations listed as included in `MVP_PROFILE.md`.

## 1. Canonical Shared Types

```agentlang
TYPE JsonValue =
    Str | Float64 | Bool | NONE |
    List[JsonValue] | Map[Str, JsonValue]

TYPE FailureDetails = Map[Str, JsonValue] | NONE
TYPE Result[T] = SUCCESS(T) | FAILURE(ErrorCode, message: Str, details: FailureDetails)

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
TYPE TokenizeResult = List[Token]
```

## 2. Included Operations (MVP-Only)

### 2.1 core.data

- `FILTER[T](items: List[T], predicate: (T) -> Bool) -> List[T]`
- `MAP[T,U](items: List[T], mapper: (T) -> U) -> List[U]`
- `REDUCE[T,A](items: List[T], seed: A, reducer: (A,T) -> A) -> A`
- `SORT[T,K](items: List[T], key: (T) -> K, order: ENUM(asc,desc)=asc) -> List[T]`
- `GROUP[T,K](items: List[T], key: (T) -> K) -> Map[K,List[T]]`
- `TAKE[T](items: List[T], n: UInt64) -> List[T]`
- `SKIP[T](items: List[T], n: UInt64) -> List[T]`

### 2.2 core.io

- `READ[T=Bytes](uri: Str, decoder: (Bytes)->T=identity) -> Result[T]`
- `WRITE[T](uri: Str, value: T, encoder: (T)->Bytes=identity, mode: ENUM(overwrite,append)=overwrite) -> Result[Hash]`
- `FETCH[T=Bytes](uri: Str, timeout: Duration=30s, decoder: (Bytes)->T=identity) -> Result[T]`

### 2.3 core.text

- `PARSE[T](input: Str, grammar: Grammar[T]) -> Result[T]`
- `FORMAT(template: Str, values: Map[Str,JsonValue], strict: Bool=TRUE) -> Result[Str]`
- `REGEX(op: ENUM(match,search,replace,split), pattern: Str, input: Str, replacement: Str=NONE) -> Result[RegexResult]`
- `TOKENIZE(input: Str, strategy: ENUM(whitespace,bpe,wordpiece,sentence)=whitespace) -> Result[TokenizeResult]`

### 2.4 core.http

- `GET[T=Bytes](url: Str, headers: Map[Str,Str]={}, timeout: Duration=30s, decoder: (Bytes)->T=identity) -> Result[HttpResponse[T]]`
- `POST[B,T=Bytes](url: Str, body: B, headers: Map[Str,Str]={}, timeout: Duration=30s, encoder: (B)->Bytes=identity, decoder: (Bytes)->T=identity) -> Result[HttpResponse[T]]`

### 2.5 agent.llm

- `GENERATE(prompt: Str, model: Str, temperature: Float32=0.2, max_tokens: UInt32=1024) -> Result[Probable[Str]]`
- `CLASSIFY[T: Enum](input: Str, labels: List[T], model: Str, min_confidence: Confidence=~0.0) -> Result[Probable[T]]`
- `EXTRACT[T](input: Str, schema: Schema[T], model: Str, min_confidence: Confidence=~0.0) -> Result[Probable[T]]`

### 2.6 agent.memory

- `REMEMBER(key: Str, value: JsonValue, scope: ENUM(task,agent,shared)=agent, ttl: Duration|NONE=NONE) -> Result[Hash]`
- `RECALL[T=JsonValue](key: Str, default: T|NONE=NONE) -> Result[T]`
- `FORGET(key: Str, scope: ENUM(task,agent,shared)=agent) -> Result[Bool]`

## 3. MVP Failure Contract

**Fallibility policy (normative):**

1. **Fallible operations** (all of `core.io`, `core.http`, `core.text`, `agent.llm`, `agent.memory`) return `Result[T]` where `T` is the success payload type. No bare-`T` returns are permitted for fallible operations.
2. **Pure operations** (`core.data`: `FILTER`, `MAP`, `REDUCE`, `SORT`, `GROUP`, `TAKE`, `SKIP`) are total functions that return bare `T`. They cannot produce `FAILURE` values.
3. All failures must use the canonical 3-field form: `FAILURE(ErrorCode, message: Str, details: FailureDetails)`. Two-field `FAILURE` forms are non-conformant for MVP v0.1.

This policy satisfies conformance requirement C3 in `README.md`.
