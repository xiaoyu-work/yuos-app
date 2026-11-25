# AI Helpers Shared-Capability Client

This group owns the complete legacy `summarize` client, not a business product,
model provider, shared Agent or replacement AI gate. Other products continue to
call the SDK directly; they do not call this App.

| Path | Responsibility |
| --- | --- |
| `apps/summarize/app.json` | Existing identity, typed `summarize.run(text)` MCP/CLI contract, AI policy and independent memory need |
| `apps/summarize/main.py` | Explicit-text validation, SDK AI request, result presentation and summary-memory request |
| `apps/summarize/server.py` | Manifest-selected public MCP entrypoint |
| `apps/summarize/test_main.py` | Original validation, call order, result, memory-bound and failure tests |
| `apps/summarize/test_mcp.py` | Staged public MCP and wire-v1 AI/policy/memory fixtures, including private-module and entrypoint refactors |
| `package.json` | Explicit capability kind, App and test selection; no native assets or provider exports |

`summarize` retains version `0.1.0`, one required text argument and no implicit
file/URL input. The AI block keeps strict safety and only `external-content`.
Monthly usage is subject to the owner-set App AI limit. The client requests at most
4,000 units per call and does not choose a provider/model or expose tools to
the model. The OS owns classification, consent and overrides, budget reservation
and settlement, input/output limits, safety, credentials and audited provider
requests. The three-line instruction is a prompt, not a new output validator.

The manifest's AI need now uses `fixed` wildcard binding to match the existing
`policy.require("ai.chat.untrusted", wild=True)` check. The old borrowing
wildcard rejected a caller's unbounded AI grant, while inheriting named-model
grants that could not satisfy that runtime check. No stored grant is rewritten,
unioned or automatically widened. A named-model grant alone remains insufficient
for this client's existing wildcard check; ordinary OS approval still governs
missing authority. AI safety and origin policy remain unchanged.

Successful results retain `summary`, `source="<input>"`, model/provider, usage,
budget and review fields. Empty model output fails. Before success returns, the
client asks the OS memory bridge to remember only the first nonblank summary
line, capped at 200 characters (197 plus `...` when needed), under
`source="summarize"`, `kind="note"` and `tags=["summarize"]`. The manifest retains
`memory.write:self:summarize`; it grants no other App's memory. Memory errors
propagate, not a success-shaped best-effort fallback. No local memory store,
user-data import or new namespace is introduced.

## Ownership and dependency contracts

The only nonstandard imports are `yuos_sdk.ai.chat`, `yuos_sdk.mcp.App`
and the existing `yuos_runtime.policy.require` / `yuos_runtime.memory.remember`
client adapters. These carry versioned App requests; OS implementation ownership
does not make their interface an entitlement reserved for bundled clients.
Every App requires the same authenticated Host, capability and sandbox contract,
with identity used for ownership/session/audit binding, not business-name
privilege. Development and staged MCP tests use `yuos-sdk/python/src` and
local `shared/python`, never a sibling OS checkout or copied provider
implementation. Installed execution uses OS libraries, without runtime
source downloads.

The OS retains the Agent, AI and memory authority, App identity/capabilities,
credentials, consent/budgets, safety/audit, sandbox and signing/install/update/
image ownership. Cross-repository runtime fixtures use complete staged payloads,
manifest-selected MCP entrypoints and public SDK wire, not private App modules
or SDK dispatchers. Compatible implementation and entrypoint refactors are
covered without OS changes. Named artifact exports, versioned runtime
compatibility and authenticated package delivery remain explicit dependencies,
not permission grants. Older SDK documentation still labels the runtime adapters
internal/bundled-only; its generic public-interface contract and existing
OS admission exceptions require coordinated follow-up. These client tests do
not establish that the entire unified integration is already implemented.

All four original files came from
[`xiaoyu-work/Yu-OS` at `de07416d4b8146d05354b8e80ed3e9f47794240e`](https://github.com/xiaoyu-work/Yu-OS/tree/de07416d4b8146d05354b8e80ed3e9f47794240e/apps/summarize).
The entrypoint and file modes are preserved. First-party Apache-2.0
attribution remains covered by the root [LICENSE](../../LICENSE).

From this repository root on Linux/WSL:

```bash
python3 tools/test.py --capability ai-helpers
python3 tools/stage.py ai-helpers --kind capability --root build/ai-helpers-stage
```

Fixtures use isolated synthetic text/data and canned public wire responses;
they make no paid/live model calls. Source completion is not full boot/upgrade,
native visual/hardware acceptance or broader backend/state/identity consolidation.
