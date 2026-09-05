# Architecture and safety decisions

## Trust boundaries

Website content, imported spreadsheets, provider responses, and AI output are untrusted. They must be validated before they enter the database or appear in outreach.

The frontend may request domain operations but may not execute arbitrary SQL, read arbitrary files, fetch arbitrary URLs, or retrieve API keys.

## Provider boundaries

`SearchProvider` will return short-lived candidates. Persisted records are created only after an official website is resolved and verified.

`AIProvider` will accept bounded reference material and a JSON schema. Website text is labeled as reference material, never as instructions. Every factual outreach statement must point to stored evidence or be explicitly labeled as an inference.

## Website retrieval

The fetch service will enforce public HTTP/HTTPS destinations, DNS and redirect revalidation, private-address blocking, robots rules, request and byte budgets, timeouts, same-site page limits, and cancellation. CAPTCHA, authentication, and access blocks end research rather than trigger circumvention.

## Outreach invariant

Opening Gmail records `gmail_opened`, not `contacted`. A sent timestamp is recorded only after the user returns and explicitly confirms sending. Do-not-contact state blocks new drafts and follow-ups.

## Portability

Application data is stored beside the executable in `data/`. WebView2's user-data directory is explicitly set to `data/webview` from Rust because a Tauri configuration-file path would otherwise be relative to AppData. The portable ZIP is preferred over installation and must be extracted to a writable D-drive directory.
