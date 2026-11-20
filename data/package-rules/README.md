# ADT Package Recommendation Rules

This directory holds the offline-first, deterministic package recommendation database for **Android Debloating Toolkit (ADT)**.

## Core Integrity Principles

1. **No fabricated recommendations or test claims.** Every `verified` rule must document real, reproducible evidence from an actual tested device or authoritative AOSP/OEM source.
2. **Conservative by default.** Unknown packages are always classified as `unknown_unverified` and are never treated as safe to disable.
3. **Deterministic precedence.** Rules are resolved in strict specificity order:
   1. Exact device model or codename rule
   2. Manufacturer-specific rule
   3. Android API-range rule
   4. General Android rule
   5. `unknown_unverified` (fallback when no active rule matches)
4. **Restrictive conflict resolution.** When equally specific active rules disagree on risk classification, the more restrictive classification always wins (`do_not_disable` > `caution` > `unknown_unverified` > `recommended_to_disable`).

## Verification Statuses

- `draft` â€” Initial author entry pending community review and device verification.
- `community_reported` â€” Reported from real-world usage, not yet independently reproduced by a maintainer.
- `verified` â€” Independently confirmed with complete `evidence` records, `lastVerifiedDate`, and `reviewer`. The Rust loader enforces this invariant and refuses to load `verified` rules that omit required evidence fields.
- `deprecated` â€” Superseded or invalidated rule retained for auditability; excluded from active recommendations.

## Required Evidence Fields (Mandatory for `verified` Rules)

Every entry in `evidence` must provide non-empty values for:

- `sourceReferences`: Documentation links, AOSP references, or issue/PR references.
- `deviceModelOrCodename`: Exact tested device model and/or codename (e.g., `SM-A528B / a52sxq`).
- `androidVersion`: Tested Android release and API level (e.g., `Android 14 (API 34)`).
- `testDate`: ISO-8601 date (`YYYY-MM-DD`).
- `observedBehavior`: Concrete behavior observed before and after disabling/re-enabling.
- `limitations`: Known caveats, OEM-skin differences, or untested features.
