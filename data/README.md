# PRUNIQ Data Licensing & Governance

Copyright © 2026 Syed Shaheer Ali (for original project-authored dataset structure, descriptions, and documentation).

## License Scope (CC-BY-4.0)

Original, project-authored content located in the following directories is licensed under the **Creative Commons Attribution 4.0 International (CC-BY-4.0)** license, whose unmodified official legal text is stored in [`data/LICENSE`](./LICENSE):

- `data/package-rules/` (package recommendation rule schema, documentation, and future original project-authored rule definitions and verification records)
- `data/app-catalog/` (curated human-readable label mappings authored for PRUNIQ)

Application source code (`src/`, `src-tauri/`, build scripts, and general documentation outside `data/`) is licensed separately under the **Apache License 2.0** (see root [`LICENSE`](../LICENSE)).

## Non-Ownership of Third-Party Names & Trademarks

CC-BY-4.0 applies **only** to original material and rights that the PRUNIQ project and its contributors are entitled to license. Nothing in this repository claims copyright or ownership over:

- Android package identifiers (for example, `com.android.*`, `com.google.*`, or OEM package namespaces)
- Manufacturer, brand, or product names and trademarks
- Application icons or third-party artwork
- Third-party product descriptions or external datasets

If any future third-party dataset or excerpt is explicitly approved for inclusion, its separate license and attribution notices will be preserved verbatim.

## Attribution Requirements

When redistributing or adapting original material from `data/package-rules/` or `data/app-catalog/` under CC-BY-4.0:

1. **Credit the project:** Attribute **PRUNIQ — Android Debloating Toolkit (Copyright © 2026 Syed Shaheer Ali)** and link to `https://github.com/shaheeralii/PRUNIQ`.
2. **Link the license:** Include a copy of or link to the CC-BY-4.0 license (`https://creativecommons.org/licenses/by/4.0/`).
3. **Indicate changes:** Clearly note any added, removed, or modified rules, risk classifications, compatibility ranges, or evidence entries, and distinguish downstream edits from upstream verified entries.
4. **No implied endorsement:** Do not imply that PRUNIQ or its contributors endorse your redistribution or modified classifications.

## Community Contributions & Evidence Integrity

- Contributors are credited through Git commit history, PR references, and optional `reviewer` and `sourceReferences` fields in contributed rules.
- `data/package-rules/rules.json` starts intentionally with **zero verified rules** (`0.1.0-initial-unpopulated`).
- External debloating lists are **not** copied or adapted without prior license review and explicit maintainer approval.
- The Rust loader (`src-tauri/src/rules/loader.rs`) rejects any rule marked `verified` unless it includes `lastVerifiedDate`, `reviewer`, and at least one complete evidence record (`sourceReferences`, `deviceModelOrCodename`, `androidVersion`, `testDate`, `observedBehavior`, and `limitations`).
