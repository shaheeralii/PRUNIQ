# Contributing to PRUNIQ — Android Debloating Toolkit

Contributions are welcome. PRUNIQ prioritizes device safety, reversibility, honest Android compatibility reporting, and evidence-backed recommendations.

## Dual-License Contribution Terms

1. **Source Code & General Documentation (`Apache-2.0`):** Contributions to Rust, TypeScript, React, configuration, scripts, and documentation outside `data/` are accepted under the [Apache License, Version 2.0](LICENSE) (Section 5).
2. **Package Rules & App Catalog Data (`CC-BY-4.0`):** Original contributions to `data/package-rules/` and `data/app-catalog/` are accepted under [Creative Commons Attribution 4.0 International](data/LICENSE). See [data/README.md](data/README.md).
3. **Third-Party Dataset Policy:** Do not copy or adapt external debloating lists without prior license review and explicit maintainer approval.

## Local Verification Checklist

Before opening a pull request, run:

```powershell
npm run typecheck
npm run test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --locked
```

Engineering guidelines:
- Keep all ADB invocations inside the closed typed allow-lists (`AdbReadOnlyCommand` and `AdbMutatingCommand`) without shell interpolation.
- Never persist raw hardware ADB serials or raw executed command strings in SQLite, logs, or exports.
- Include unit or mock integration tests for parser, rule-engine, protection, recovery, or storage changes.
- Clearly distinguish mock-tested behavior from physical-device verification in PR descriptions.

## Package Rule Contributions

Every rule in `data/package-rules/rules.json` must match [`data/package-rules/schema.json`](data/package-rules/schema.json) and pass `cargo test`.

- Never mark a rule `verified` without real supporting evidence (`sourceReferences`, `deviceModelOrCodename`, `androidVersion`, `testDate`, `observedBehavior`, `limitations`), `lastVerifiedDate`, and `reviewer`.
- Never fabricate device test claims or mark unknown packages safe by default.
- See [`data/package-rules/README.md`](data/package-rules/README.md).
