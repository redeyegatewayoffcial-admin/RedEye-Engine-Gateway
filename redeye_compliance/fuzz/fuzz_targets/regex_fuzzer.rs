//! fuzz_targets/regex_fuzzer.rs — Native Regex Engine Fuzzer
//!
//! # What This Fuzzer Tests
//!
//! Feeds raw, arbitrary UTF-8 strings directly into the `PresidioGrpcClient`
//! (the production regex-based Presidio stub) *without* going through the
//! JSON layer.  This isolates the regex engine itself so the fuzzer can find:
//!
//!   - Catastrophic Backtracking (ReDoS) in the compiled PII patterns.
//!   - Panics in `regex::Regex::find_iter` on unusual Unicode input.
//!   - Off-by-one errors in byte offsets returned by `collect_matches`.
//!   - Unexpected interactions between multi-byte Unicode codepoints and
//!     the byte-indexed entity spans.
//!
//! NOTE: The `regex` crate uses a linear-time NFA engine and is generally
//! resistant to ReDoS.  However, this fuzzer will find any future regressions
//! if the patterns are ever changed to use backtracking constructs.
//!
//! # Architecture
//!
//! We call `PresidioAnalyzer::analyze()` directly via the trait, bypassing
//! `PiiEngine::scan()` entirely.  This is intentional — it gives us maximum
//! isolation and lets the fuzzer maximize coverage of the regex matching paths.

#![no_main]

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use libfuzzer_sys::fuzz_target;

use redeye_compliance::error::AppError;
use redeye_compliance::usecases::pii_engine::{
    PresidioAnalyzer, PresidioEntity, PresidioGrpcClient,
};

// ── Runtime Helper ────────────────────────────────────────────────────────────

fn make_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|_| std::process::abort())
}

// ── Fuzz Target ───────────────────────────────────────────────────────────────

fuzz_target!(|data: &[u8]| {
    // ── Stage 1: convert bytes to a valid UTF-8 string ──────────────────────
    //
    // The `regex` crate operates on `&str`, so we must have valid UTF-8.
    // If the fuzzer provides invalid bytes, we skip the iteration silently.
    // This is the correct pattern per the architecture constraints.
    let text = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };

    // ── Stage 2: construct the production Presidio client ───────────────────
    //
    // `PresidioGrpcClient::new()` compiles all regex patterns.  If this
    // fails, the host is unable to compile valid regex — a hard failure.
    // We abort so that libFuzzer surfaces this as a crash, not a silent skip.
    let client = match PresidioGrpcClient::new() {
        Ok(c) => c,
        Err(_) => {
            // Regex compilation failure is a real bug — abort so libFuzzer
            // reports it.
            std::process::abort();
        }
    };

    // ── Stage 3: run the analyzer ────────────────────────────────────────────
    let rt = make_runtime();
    let result = rt.block_on(client.analyze(text));

    // ── Stage 4: validate entity byte spans ──────────────────────────────────
    //
    // Every entity returned by the analyzer MUST have:
    //   (a) start <= end
    //   (b) end <= text.len()  (text is a byte slice, spans are byte offsets)
    //   (c) text[start..end] is valid UTF-8  (must not bisect a codepoint)
    //
    // If any of these invariants are violated, the redaction loop in
    // `PiiEngine::scan()` would panic when slicing the text — a real bug.

    match result {
        Ok(entities) => {
            for entity in &entities {
                // (a) start ≤ end
                assert!(
                    entity.start <= entity.end,
                    "INVARIANT VIOLATION: entity '{}' has start({}) > end({})",
                    entity.entity_type,
                    entity.start,
                    entity.end,
                );

                // (b) end ≤ text.len()
                assert!(
                    entity.end <= text.len(),
                    "INVARIANT VIOLATION: entity '{}' end({}) > text.len()({})",
                    entity.entity_type,
                    entity.end,
                    text.len(),
                );

                // (c) the slice must be valid UTF-8 (cannot bisect a codepoint)
                // If `text` is valid UTF-8 and the regex engine returns byte
                // offsets that land on character boundaries, this will never
                // panic in production — but this assertion will catch it if
                // a future regex change breaks that assumption.
                let _slice = &text[entity.start..entity.end];
                // No need to call is_char_boundary explicitly; the slice
                // indexing above panics if the bounds bisect a codepoint.

                // (d) confidence score must be in [0.0, 1.0]
                assert!(
                    (0.0..=1.0).contains(&entity.score),
                    "INVARIANT VIOLATION: entity '{}' score({}) out of [0, 1]",
                    entity.entity_type,
                    entity.score,
                );
            }
        }
        // An Err from analyze() is a graceful rejection — not a bug.
        Err(_) => {}
    }
});
