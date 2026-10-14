# Playbook: Issue 03 — Asset Formats: IFF Chunk Hierarchy Parser & STF Localized String Table Reader (`swg-client-iff`)

> **Target Issue:** Bayly-AI/StarWars-Galaxies-Client#3  
> **Branch:** `enhancement/3-iff-chunk-hierarchy-parser-stf`  
> **Status:** Implementation Playbook

---

## 1. Objectives & Scope

Implement core binary asset format parsers in `crates/swg-client-iff`:

1. **Recursive FORM / CHNK IFF Reader & Serializer (`iff.rs`):**
   - FourCC identifier representation and big-endian size handling.
   - Zero-allocation slice-based traversal for nested `FORM` containers and leaf chunks (`DATA`, `INFO`, `0000`, etc.).
   - Cursor-based hierarchical navigator (`IffCursor`) with `enter_form`, `exit_form`, `next_chunk`, and predicate searches.
   - Bidirectional serializer (`IffWriter`) for asset authoring and test generation.

2. **STF Localized String Table Reader & Lookup Dictionary (`stf.rs`):**
   - Binary `.stf` parser decoding UTF-16LE localized string values and ASCII string identifiers.
   - Dual-index lookup table (`StringTable`) enabling both key-based (`@stf_file:string_id`) and integer ID lookups.
   - `.stf` binary encoder for round-trip testing and localization pipeline tooling.

---

## 2. Acceptance Criteria & Quality Gates

- [x] Accurately parses recursive FORM/CHUNK hierarchies without heap reallocations during traversal.
- [x] Decodes UTF-16LE `.stf` string tables into dual key/ID lookup maps.
- [x] Full round-trip serialization tests for both IFF and STF formats.
- [x] Full workspace tests pass (`cargo test --workspace`) and zero clippy warnings (`cargo clippy --workspace -- -D warnings`).
- [x] `hath0r preflight run` passes all gates.
