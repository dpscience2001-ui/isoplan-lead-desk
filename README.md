# IsoPlan Lead Desk

IsoPlan Lead Desk is a private Windows application for researching a small number of qualified vacation-rental management companies and preparing evidence-grounded outreach for Diptarko Mukherjee's 3D floor-plan service.

The product is deliberately not a mass-email scraper. It never scrapes Google Maps, guesses addresses, bypasses blocked content, or sends email automatically.

## Current status

The foundation and core manual-workflow slices are implemented. The repository contains the Tauri/React shell, versioned SQLite schema, manual lead entry, local CSV/XLSX import, duplicate protection, evidence review and scoring, public-contact records, notes, dashboard counts, pipeline controls, local settings, backups/exports, and persistent do-not-contact enforcement. External discovery, AI calls, and sending are not active.

## Manual imports

CSV and `.xlsx` imports are parsed locally in the application and are never uploaded. Files are limited to 10 MB, 2,000 rows, 100 columns, and the first Excel worksheet. The importer recognizes the supplied tracker headings such as `Companyname` and `Website`, previews valid rows, asks for a fallback market when country is absent, and reports duplicates and invalid rows.

The fallback country must be reviewed carefully: the original tracker contains no country column, so the software cannot infer a market reliably from that file alone.

## Lead scoring model

Scores are calculated in application code from stored evidence, not requested as an unexplained AI number. Each category contributes at most once:

| Category | Maximum points |
| --- | ---: |
| Relevant management business | 20 |
| Portfolio approximately 5–50 properties | 15 |
| Public business contact | 10 |
| No floor plan found on analyzed pages | 15 |
| Limited 2D floor-plan presentation | 15 |
| Multi-unit or whole-property layout | 15 |
| Premium visual marketing | 5 |
| Repeat-work potential | 10 |
| Supported geography | 5 |

Disqualifying evidence subtracts 25 points for strong existing 3D plans, 25 for no lawful public contact, or 40 for an individual host. The final result is clamped to 0–100. Confidence depends on the number and quality of evidence items. Every contribution remains visible with its classification, confidence, exact source URL, and supporting observation.

## Backup and export

Database snapshots use SQLite's `VACUUM INTO` mechanism and are written to `data/backups` beside the portable executable. The configured retention count is bounded to 1–30. Lead exports are written to `data/exports` as CSV or JSON. CSV values beginning with spreadsheet formula characters are prefixed safely, and provider credentials are never part of the database or export.

Restore is intentionally not exposed until native integration tests can verify validation, pre-restore backup creation, database replacement, and restart behavior end-to-end.

## Outreach drafts

Initial drafts require a published email contact plus at least one sourced, positive, non-AI evidence item. English and French seed templates insert the company, selected observation, central Fiverr Gig URL, sender identity, and country-language opt-out line. Drafts remain editable, and editing always clears previous approval.

Approval records an audit event but sends nothing. Deceptive initial subjects beginning with `Re:` are rejected. Gmail composition and sent confirmation remain separate workflow states; opening Gmail must never be treated as proof that an email was sent.

## Architecture

- Tauri 2 provides the Windows desktop shell.
- React and TypeScript provide the interface.
- Rust owns SQLite, filesystem, credential, network, and operating-system access.
- SQLite stores leads, evidence, sources, contacts, drafts, settings, and audit events.
- Search and AI integrations will sit behind replaceable provider interfaces.

The rendered interface does not receive unrestricted database, filesystem, or secret access. Privileged operations are exposed as narrow Tauri commands.

## Storage location

This repository and all repository-local caches belong on the `D:` drive. The application uses a portable `data` directory beside its executable rather than Windows AppData on `C:`. Its WebView2 cache is redirected to `data/webview` beside the executable as well. The preferred release is a portable ZIP: extract it to D: and run it without an installer.

The database and user backups are not part of the application-size budget because they grow with use. Website images and complete page archives will not be retained by default.

## Size budget

The release gate is less than **150 MB installed**, excluding:

- the Microsoft WebView2 runtime already shared by Windows;
- the user's lead database; and
- user-created backups and exports.

The expected portable application size is 40–90 MB. Remote builds enforce a hard failure at 150 MB. An NSIS installer may be offered later, but the portable ZIP avoids nearly all installer bookkeeping on C:.

## Development prerequisites

- Windows 10 or 11 with WebView2
- Node.js
- Rust stable MSVC toolchain
- Microsoft Visual C++ Build Tools with Desktop development for C++

The present machine has Node.js and Git but does not currently expose Rust or Cargo. Installing system build prerequisites is intentionally not performed automatically.

## Repository-local commands

PowerShell script execution currently blocks `npm.ps1`, so use `npm.cmd`:

```powershell
$env:npm_config_cache = "$PWD\.npm-cache"
npm.cmd install
npm.cmd run typecheck
npm.cmd run test
npm.cmd run tauri dev
```

For frontend-only work, `scripts/Invoke-LocalFrontend.ps1` redirects npm cache and temporary storage to this repository on D:. Project dependencies naturally remain in the local `node_modules` directory; the checked-in `.npmrc` keeps npm's download cache local as well.

## Building without a local toolchain

`.github/workflows/windows-portable.yml` builds the Windows executable on a GitHub-hosted Windows runner. It runs type checking, tests, the security audit, and the 150 MB size gate before publishing a portable ZIP for seven days. This prevents the Rust and Visual Studio build toolchains from consuming space on the local C: drive.

Do not put production API keys in `.env`. Provider keys will be saved through Windows Credential Manager and excluded from logs, databases, backups, and exports.

## Initial database model

Migration `001_initial.sql` creates normalized records for:

- leads and pipeline status;
- public business contacts;
- exact source pages;
- evidence claims and their provenance;
- reviewed outreach drafts;
- audit timeline events;
- persistent do-not-contact entries; and
- non-secret application settings.

Migrations are applied transactionally and recorded in `schema_migrations`. Existing databases will not be destructively recreated.

## Planned delivery order

1. Desktop shell and local database
2. Manual URL and spreadsheet import
3. Review, qualification, pipeline, and do-not-contact workflows
4. Safe official-website research
5. Optional compliant search-provider adapter
6. English and French outreach drafting
7. Gmail compose and explicit sent confirmation
8. Follow-ups, packaging, security review, and verification

## External services

No external service is needed for local lead management. Later stages may optionally use:

- an AI provider for structured research and drafting; and
- Brave Search API for discovery, only after the selected plan's result-storage terms are accepted.

Provider failure must never prevent access to the local lead database.

## Known limitations

- Rust and the Windows C++ build toolchain are not yet available, so the native shell has not been compiled.
- The current interface is a foundation view; navigation and database-backed workflows are not connected yet.
- Application icons and installer assets will be added after the product UI is stable.
