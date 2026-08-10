# Transfer pipeline — next cycle

Implementer note after the process-first copy/move API. Captures what shipped, what the queue is not, and deferred work—especially the bounded RAM read-ahead pool.

## What shipped

- **`TaskQueue` of path/size work items** — producers prepare discrete `TransferTask`s (source/dest paths, size, index); a **single writer** consumer performs file body I/O.
- **`ProcessSession`** (core) — cancel, event emit, queue helpers for the engine.
- **`StorageProcess`** (runtime / .NET) — public awaitable executioner: work starts immediately; sync copy/move waits inside; async returns the process (`Future` / `Completion` + `GetAwaiter`).
- **`StorageProcessEvent` stream** — `Started` / `CurrentItem` / `Bytes` (throttled) / terminal; not fat percentage snapshots.
- Content intelligence stays on **analyze / metadata** APIs only (not during copy).

## What the queue is not

| Not this | Reality |
|----------|---------|
| Future / Promise / `Task` | Queued items are **work descriptors**, not awaitables. The awaitable is **`StorageProcess`**. |
| Byte buffer / RAM staging | Queue holds **path + size**, not file body bytes. Read+write of content still happens on the single writer thread. |

## Bounded RAM byte staging / read-ahead pool (next task)

**Goal:** configurable RAM limit; readers fill slabs ahead of the single writer so steady-state I/O can be write-bound (especially cross-volume). First stretch may still couple read+write per file; keep **one writer** (HDD thrash avoidance).

Sketch relative to today:

1. Keep `TaskQueue<TransferTask>` for ordering / prep.
2. Add a bounded **byte pool** (or slab allocator) fed by optional reader workers.
3. Writer pulls prepared slabs instead of always `read`→`write` in lockstep.
4. Wire cancel and process events through the same `StorageProcess` / session.

Plan this as the **next cycle** after process-first API lands.

## Integrity / checksum verify

TeraCopy-shaped verify (post-copy or streaming checksums) is a separate follow-on. It is **not** content-intelligence analysis. Still deferred.

## Broader process-first surface

This cycle is **copy/move only**. Read/write/watch as processes can come later using the same handle shape.

## C# note

Compose with `Task` / `TaskCompletionSource` and `GetAwaiter()` on `StorageProcess`. **Do not** subclass `System.Threading.Tasks.Task`.
