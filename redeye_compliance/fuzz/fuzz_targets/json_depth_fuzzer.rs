//! fuzz_targets/json_depth_fuzzer.rs — Recursive JSON Structure Fuzzer
//!
//! # What This Fuzzer Tests
//!
//! Specifically targets the two recursive functions that walk the entire JSON
//! tree without a depth bound:
//!
//!   - `collect_text(value, buf)` — called through `extract_all_text()`
//!   - `apply_replacements(value, replacements)` — the mutable tree walk
//!
//! An adversary can craft a deeply nested JSON payload (`{"a":{"a":{"a":...}}}`)
//! that causes an unbounded recursive call stack → **stack overflow → SIGABRT**.
//! This fuzzer will confirm whether the current implementation is vulnerable.
//!
//! # How It Works
//!
//! Rather than using `Arbitrary`, this target manually constructs worst-case
//! JSON shapes directly from the raw byte stream:
//!   - Byte 0 → nesting depth (0–255)
//!   - Byte 1 → shape selector (object / array / mixed)
//!   - Remaining bytes → string content injected at the leaf
//!
//! This means the fuzzer will quickly discover the exact depth at which a
//! stack overflow occurs (if any), and the crash corpus will be trivially
//! reproducible.

#![no_main]

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use libfuzzer_sys::fuzz_target;
use serde_json::{json, Value};

use redeye_compliance::error::AppError;
use redeye_compliance::usecases::pii_engine::{
    PiiEngine, PresidioAnalyzer, PresidioEntity,
};

// ── Null-I/O Presidio Mock (same as pii_fuzzer.rs) ───────────────────────────

struct NullPresidio;

impl PresidioAnalyzer for NullPresidio {
    fn analyze<'a>(
        &'a self,
        _text: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PresidioEntity>, AppError>> + Send + 'a>> {
        Box::pin(async { Ok(Vec::new()) })
    }
}

// ── JSON Builders ─────────────────────────────────────────────────────────────

/// Build a deeply nested object: `{"a": {"a": {"a": ... leaf ... }}}`
fn build_nested_object(depth: usize, leaf: &str) -> Value {
    if depth == 0 {
        return Value::String(leaf.to_string());
    }
    json!({ "a": build_nested_object(depth - 1, leaf) })
}

/// Build a deeply nested array: `[[[[... leaf ...]]]]`
fn build_nested_array(depth: usize, leaf: &str) -> Value {
    if depth == 0 {
        return Value::String(leaf.to_string());
    }
    Value::Array(vec![build_nested_array(depth - 1, leaf)])
}

/// Build an alternating object/array nest.
fn build_mixed_nest(depth: usize, leaf: &str) -> Value {
    if depth == 0 {
        return Value::String(leaf.to_string());
    }
    if depth % 2 == 0 {
        json!({ "level": build_mixed_nest(depth - 1, leaf) })
    } else {
        Value::Array(vec![build_mixed_nest(depth - 1, leaf)])
    }
}

/// Build an extremely wide flat object (many keys at the same level).
fn build_wide_object(width: usize, value_template: &str) -> Value {
    let mut obj = serde_json::Map::new();
    for i in 0..width {
        obj.insert(format!("key_{}", i), Value::String(value_template.to_string()));
    }
    Value::Object(obj)
}

// ── Runtime Helper ────────────────────────────────────────────────────────────

fn make_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|_| std::process::abort())
}

// ── Fuzz Target ───────────────────────────────────────────────────────────────

fuzz_target!(|data: &[u8]| {
    if data.len() < 3 {
        return;
    }

    // Byte 0: nesting depth, clamped to [1, 200].
    // 200 levels is deep enough to trigger a stack overflow if the recursion
    // is unbounded but shallow enough that the *builder* functions above stay
    // within the host's default stack.
    let raw_depth = data[0] as usize;
    let depth = raw_depth.max(1).min(200);

    // Byte 1: shape selector.
    let shape = data[1] % 4;

    // Byte 2: extra trigger flag.
    let inject_ssn_keyword = data[2] % 2 == 1;

    // Remaining bytes: leaf string content (UTF-8, reject invalid bytes).
    let leaf = match std::str::from_utf8(&data[3..]) {
        Ok(s) => s,
        Err(_) => return, // not a bug — just invalid input
    };

    // Optionally embed an SSN keyword so Tier 1 fires (exercises escalation).
    let leaf_content = if inject_ssn_keyword {
        format!("ssn {}", leaf)
    } else {
        leaf.to_string()
    };

    // Construct worst-case JSON shape.
    let payload = match shape {
        0 => build_nested_object(depth, &leaf_content),
        1 => build_nested_array(depth, &leaf_content),
        2 => build_mixed_nest(depth, &leaf_content),
        3 => {
            // Wide-flat: up to 1024 keys.
            let width = (data[0] as usize * 4).min(1024);
            build_wide_object(width, &leaf_content)
        }
        _ => unreachable!(),
    };

    // Build engine with the null mock (we're stress-testing the traversal,
    // not the regex matching).
    let engine = match PiiEngine::with_analyzer(Arc::new(NullPresidio)) {
        Ok(e) => e,
        Err(_) => return,
    };

    let rt = make_runtime();
    // Any panic inside scan() will be caught by libFuzzer and reported as a
    // crash. SIGABRT from a stack overflow is also captured.
    let _result = rt.block_on(engine.scan(payload));
    // We don't assert on the result — a graceful Err is fine.
    // We're only interested in panics and aborts.
});
