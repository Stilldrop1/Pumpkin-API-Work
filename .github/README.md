# Pumpkin API Work

**Proposed fixes and speed-ups for Pumpkin's new v0.2 plugin event system.**

This repository builds on [Pumpkin](https://github.com/Pumpkin-MC/Pumpkin) pull request [#4609](https://github.com/Pumpkin-MC/Pumpkin/pull/4609), which redesigns how plugins receive events. While reviewing it, I found several bugs worth fixing before the v0.2 plugin API is released. This repo contains those fixes as 9 small, separate commits, plus a benchmark showing the result.

**[See only our changes compared to PR #4609 →](https://github.com/Stilldrop1/Pumpkin-API-Work/compare/upstream-pr-4609...main)**

> Not an official Pumpkin project. It's an independent proposal, shared for review and discussion.
---

## At a glance

| | |
|---|---|
| **Based on** | Pumpkin PR #4609 (`752e17c`), branch [`upstream-pr-4609`](https://github.com/Stilldrop1/Pumpkin-API-Work/tree/upstream-pr-4609) |
| **Changes** | 9 commits: 7 bug fixes, 1 cleanup, 1 performance improvement |
| **Speed** | Firing an event is **12–29% faster** than in the PR |
| **Compatibility** | No breaking changes for plugin authors |
| **Tests** | All lint, format and test checks pass on the crates that were changed |

---

## The problems

PR #4609 is a good change. Instead of one large blob, plugins now get an event object with getters and setters. But in its current state:

- The server can **crash** when a plugin handles an event.
- A plugin that **crashes halfway through** can leave an event half-changed.
- Some setters **say "done" but quietly throw the change away**.
- Event **priorities have no effect**.
- **CI fails**, so the PR hasn't been fully tested.

## The fixes

| # | Change | What it means | Commit |
|---|---|---|---|
| 1 | Handlers can't crash the server | If an event is briefly busy or the wrong type, the handler waits or is skipped instead of crashing the server. | [`8ab834f`](https://github.com/Stilldrop1/Pumpkin-API-Work/commit/8ab834f05) |
| 2 | Crashing plugins leave no mess | A Wasm plugin's changes only take effect if it finishes successfully. | [`16ab319`](https://github.com/Stilldrop1/Pumpkin-API-Work/commit/16ab31911) |
| 3 | CI passes again | Lint errors fixed, and the test bot updated to the new event system. | [`3516925`](https://github.com/Stilldrop1/Pumpkin-API-Work/commit/3516925b0) |
| 4 | Cleaner internals | Event storage uses named fields instead of `.0` / `.1`, as a reviewer asked. | [`820f99b`](https://github.com/Stilldrop1/Pumpkin-API-Work/commit/820f99bc8) |
| 5 | Priorities work | Handlers run from Lowest to Highest, so Highest gets the final say. | [`dd5f3de`](https://github.com/Stilldrop1/Pumpkin-API-Work/commit/dd5f3de39) |
| 6 | Setters tell the truth | Edits the server can't apply now return an error instead of being silently ignored (details below). | [`ef99f6f`](https://github.com/Stilldrop1/Pumpkin-API-Work/commit/ef99f6f06) |
| 7 | Review follow-ups | Closes a rare item-loss case, keeps a noisy log message out of normal logs, and makes one test fully reliable. | [`b2510b1`](https://github.com/Stilldrop1/Pumpkin-API-Work/commit/b2510b1be) |
| 8 | Faster dispatch | Removes lock and unlock steps that weren't needed on every event. | [`0658352`](https://github.com/Stilldrop1/Pumpkin-API-Work/commit/06583529d) |
| 9 | Busy events don't kill plugins | A plugin now gets a normal "unavailable" error instead of being stopped. | [`9d4ba92`](https://github.com/Stilldrop1/Pumpkin-API-Work/commit/9d4ba92b0) |

<details>
<summary><b>More detail on fix 6: setters that ignored changes</b></summary>

- **Inventory click:** setting the event wiped out the clicked item and the cursor item, and leaked memory. Both items are now kept, and nothing leaks.
- **Server list ping:** changes to the hostname or address did nothing, because they describe the incoming connection. The setter now returns an error. Changing the MOTD, player counts and icon still works.
- **Piglin bartering:** changes to the items the piglin gives back were ignored. They now apply, and bad input from a plugin is rejected instead of crashing the server.

</details>

---

## Performance

Time to fire one event (lower is better). All three versions were run on the same machine with the same settings, on a real server instance.

| Situation | Original PR | After fixes 1–7 | **Final** | Change |
|---|---:|---:|---:|---:|
| 1 handler | 260 ns | 269 ns | **186 ns** | **29% faster** |
| 5 handlers | 800 ns | 834 ns | **646 ns** | **19% faster** |
| 5 handlers + 5 listeners | 1.52 µs | 1.37 µs | **1.21 µs** | **20% faster** |
| 5 handlers, one keeps its event handle | 766 ns | 801 ns | **672 ns** | **12% faster** |

The safety fixes on their own cost about 3–5%. Fix 8 more than wins that back.

- The saving per event is small, but it adds up on events that fire constantly, such as player movement and server ticks.
- Each version was measured once, and only native plugins were measured, not Wasm plugins.
- The numbers were taken before fix 9, which only affects the rare "event is busy" case.
- The benchmark is in [`.github/benchmark`](benchmark/) if you want to reproduce it.

---

## Does this break anything?

**No.** The plugin interface (WIT) is unchanged beyond what PR #4609 already does. Existing native plugin code keeps compiling.

These behaviour changes are intentional:

- Handlers run in priority order.
- Some setters return an error instead of silently ignoring a change.
- If a Wasm plugin crashes, its changes to the event are discarded.
- A busy event gives a plugin an error instead of stopping it.

## How it was tested

- ✅ `cargo clippy --workspace --all-targets --all-features` with warnings treated as errors
- ✅ `cargo fmt --check`
- ✅ All tests pass for every crate that was changed, including the Wasm plugin component tests, plus new tests for fixes 1, 5 and 6

**Not yet tested:**

- A real Wasm plugin running on a live server from start to finish.
- In-game play, or bots.
- Linux and Android builds.

## What's left

1. **Catch up with Pumpkin's `master`.** There are 9 merge conflicts, mostly from `PlayerTransferEvent`, which needs converting to the new event design.
2. **Merge the base PR [#4606](https://github.com/Pumpkin-MC/Pumpkin/pull/4606) first.**
3. **Update PR #4609's description.** It still describes code that no longer exists.
4. **Test end to end** with a real plugin that cancels an event, changes an event, and crashes on purpose.

## Open questions for Pumpkin maintainers

1. Should the **Highest** priority handler run **last** (and win), as it does here, or first?
2. Is it OK for setters to **return an error** when a change can't be applied?
3. Would you like these as **extra commits on #4609**, or as a **separate PR**?

---

## Repository layout

| Branch | Contents |
|---|---|
| [`main`](https://github.com/Stilldrop1/Pumpkin-API-Work/tree/main) | PR #4609 + the 9 fixes + this page |
| [`upstream-pr-4609`](https://github.com/Stilldrop1/Pumpkin-API-Work/tree/upstream-pr-4609) | The original PR, unchanged, for comparison |

## Credit and license

All credit for Pumpkin goes to the [Pumpkin contributors](https://github.com/Pumpkin-MC/Pumpkin/graphs/contributors), and for the event redesign to the author of PR #4609. This repository is a modified copy of Pumpkin and keeps Pumpkin's original licenses: GPL-3.0 for the server, and MIT or Apache-2.0 for the plugin API crates. See [`LICENSE`](../LICENSE).
