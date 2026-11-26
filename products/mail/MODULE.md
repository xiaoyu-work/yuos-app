# Mail Product

## Responsibility

Own Mail's native Thunderbird source, shared AI business operations, extension
UI and product asset assembly. The OS owns privileged launch, App sessions,
model policy and signed installation.

| Path | Responsibility |
| --- | --- |
| `comm/` | Full Thunderbird source; native account, message and compose engine |
| `apps/mail-ai/` | Existing six shared AI operations and manifest |
| `apps/email/` | Thunderbird-backed Mail MCP adapter, stable objects, arrival cursors and submission receipts |
| `extension/` | AI UI plus native mailbox business operations using Thunderbird's existing account/message APIs |
| `build.py`, `mozconfig`, `upstream.json` | Matched native source build |
| `build-extension.py`, `package.json` | Deterministic XPI and installed asset ownership |
| `PROVENANCE.md` | Upstream revisions, licenses and trademark obligations |

The native product build is independent of the OS Rust workspace. Keep
provider credentials and authorization in the OS; importing source does not
grant access to a user's profile. Retained `mail-ai` and `email` identities and
paths remain separate. The legacy `gateway-email` App is explicitly retired;
its source and active installation selection are removed, not aliased to Email.
Existing credentials and installed user data are not deleted or migrated.
Email operates on the owner's signed-in Thunderbird accounts. Mailbox calls
are product-owned, not grantable SDK permissions. Filesystem, event, App
storage and AI/memory needs still use their own grants.

## Activity message objects

`email` declares `thunderbird-message` and `thunderbird-draft` object types.
Their IDs encode the profile-qualified account, native folder and RFC Message-ID,
not Thunderbird's numeric IDs, which change on restart or movement.
`native_message` and `native_draft` resolve through Mail's private dispatch.
The exact profile and account encoded in the ID are still checked.
Read/search/list results include canonical `app://email/TYPE?id=...` references.
References do not fetch data, grant access or merge accounts. Moved, missing
or ambiguous messages fail explicitly. Old provider object types/arguments
are retired, not aliased to unrelated Thunderbird messages.

```bash
yu app email read --account ACCOUNT_ID --id MESSAGE_REFERENCE_ID
yu activity attach-object ACTIVITY_ID --label "Source email" \
  --app email --type thunderbird-message --object-id MESSAGE_REFERENCE_ID
```

## Existing transport and packaging

The Mail desktop package installs the Yu Mail AI XPI into the distribution's
Thunderbird. The XPI owns the Assistant, options, and AI popups in
`extension/ui/`; their shared styles live in `extension/lib/ui.css`.
Thunderbird still owns the inbox, native compose window, and window controls.
The checked-in `comm/` source build is not the desktop package's runtime.

The Mail desktop package also ships `thunderbird/yuos-mail.*`, a native
Thunderbird 140 ESR presentation overlay. Packaged defaults choose Vertical
layout and Cards for new profiles; existing layout choices are not locked
or rewritten. The overlay defaults to 13px UI text and proportional plain-text
mail, using system-installed Noto fonts when available. Thunderbird's own font
preferences are not locked. AutoConfig loads the package-owned stylesheet from
`distribution/`, not from a URL, and does not lower the AutoConfig sandbox.
It is not a theme extension or part of the Web App.
Without an account, the window opens directly on Thunderbird's own account
setup page, with its explanatory text and illustration hidden; closing that
page returns to the native window.
On Thunderbird versions other than 140 the custom overlay is not loaded.
The standard UI remains available by setting `yuos.mail.chrome.enabled` to
`false` in Thunderbird's Config Editor and restarting. Administrators using
a different site AutoConfig should keep their own configuration.

The signed Email Web entry in `web/` presents Thunderbird's existing accounts
and folders in a three-pane mailbox with its own search, command bar, list
and reading area. New mail and replies use the same draft review tools.
This presentation does not replace the distribution Thunderbird window,
change the active Thunderbird profile, or add account providers.

`sync` reads the extension's bounded native inbox-arrival journal without a
model request. The first pass records a non-replaying baseline; subsequent
passes publish `message.received` references before advancing the App-private
account cursor. Expired history requires explicit `--reset`; corrupt state
is not repaired or silently replaced. The old provider continuation files
remain untouched. An OS cron can poll independently of Agent execution;
authorized Activity triggers still consume `app:email` events.
Mail mutations publish reported events through the same SDK namespace.
Event failures preserve the original operation result and never repeat its
external effect. Exact `ipc.publish:app:email` needs are disclosed separately.

`draft_save` saves a real draft through Thunderbird's background message API.
Replacing a draft additionally requires its explicit matching `source-account`.
The sender's configured Drafts folder can
belong to another mailbox; returned references identify that actual mailbox.
`native_draft` exposes To/Cc/Bcc, text/HTML and attachment byte fingerprints.
`draft_send` requires the complete current review, explicit confirmation,
and an explicit `sender-account` from the same Thunderbird profile.
The bytes and recipients sent come from the captured reviewed snapshot.
Only an unchanged source draft is removed after successful save/send;
concurrent edits or cleanup failures retain it and are reported explicitly.
Confirmation is neither an OS permission grant nor delivery proof.
The existing `email.draft` remains an AI text helper and never saves or sends.

Outbound `send`, `draft_save` and `draft_send` optionally accept a canonical
UUID `request-id`. The owned `mail-requests` partition durably records the
canonical operation/argument fingerprint before submission and the result
afterward. Identical retries return the retained result; different arguments,
corruption and interrupted in-flight requests never resubmit. Read/write
authority for this store is conditional on supplying the flag. Without it,
the operation has no retained request receipt. This is an App request receipt, not provider
exactly-once delivery or an OS-confirmed effect. Activity workflows should
retain one request ID per reviewed outward action.

`attachment_download` takes an explicit mailbox account, stable message ID,
native MIME-part ID and canonical output path, bounded to 16 MiB. Output
parents must exist; private atomic creation never overwrites an existing
entry. Failed durability after creation is reported as indeterminate.
Attachment filenames never select a destination. `attachment-file` references
resolve through `attachment_file` with only exact metadata authority, while
downloads require exact `fs.write`. The OS persists bytes
through its existing pinned non-overwriting filesystem target and task-bound
snapshot path. Attachment bytes do not cross the Python SDK response limit.

Email calls `yu __mail request --request-stdin` using the SDK's existing
explicit-binary/cancellable transport. The wire is private to authenticated
`email` Host dispatch; it is not a public mailbox client. `yuosd` validates
closed requests, the owner and active Thunderbird profile. No caller can
choose a provider socket, UID, profile path or executable.
The signed Email App tree carries `mail-private-protocol.json` with protocol
version 2. The matching XPI announces version 2 in `mailbox.ready`; request,
response and AI-only message shapes are unchanged.
Standalone CLI mailbox operations and same-App Web tools are retained, and
must both reach the verified OS Host. Ordinary relay handles cannot use this
private dispatch. Verify both paths against the matching OS before publishing.
When needed it starts fixed Thunderbird through the owner's service manager,
without the Agent UI or model loop. Installed session/display prerequisites
still apply; unavailable services are explicit errors.
The OS-owned Native Messaging multiplexer admits only root broker mailbox
requests. AI App stdout can return only correlated AI responses, never mailbox
commands. The `mail-ai` host's standing AI/memory ceiling stays unchanged.
Email's `_shared` dependency is an App-owned common library under
[`shared/python`](../../shared/MODULE.md), not another App or an OS export.
Source-checkout Native Messaging uses the checked-in `yuos-sdk/python/src`,
never a sibling OS checkout. The isolated native host never downloads build
dependencies at runtime and disables bytecode when importing repository sources.
Installed execution retains
the fixed `/usr/lib/yuos/python` root and existing OS authority launcher.
Protocol tests stage their own source/bootstrap fixture and a fully verified
platform archive selected by the developer test command. The fixture's synthetic
pin is private test data, never a production pin or runtime override; the actual
Native Host continues to exercise its unchanged pinned-source bootstrap.

The OS launcher now runs the existing declared `native-host` stdin operation
through the ordinary authenticated App Host and sandbox. The original protected
launcher/interpreter/Thunderbird-parent checks remain at the OS transport
boundary; they confer no App privilege. The App receives only its operation's
declared AI/memory needs, never a union of MCP tools or mailbox rights.
Legacy native registration is retired. The OS-owned mailbox multiplexer
retains root-peer admission and correlated AI-only child output. This cutover
does not make the source-bootstrap/protocol tests full installed Mail, desktop
or publication acceptance.

From the repository root, use `python3 tools/test.py mail`. For native tests,
build first and use `python3 products/mail/build.py test comm/path/to/test.js`.
Contract fixtures do not replace native engine, GUI or package-upgrade tests.
