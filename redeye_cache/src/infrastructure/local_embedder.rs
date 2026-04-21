//! infrastructure/local_embedder.rs
//!
//! Provides a thread-safe, locally-executed ONNX embedding engine using
//! `fastembed`. This is the **exact same model** as the L1 in-memory cache
//! in `redeye_gateway` (BGESmallENV15 → 384-dimensional vectors), ensuring
//! both cache tiers operate in the same latent vector space.
//!
//! Design:
//!  - `LocalEmbedder::new()` is called once at service boot. If the ONNX
//!    model cannot be loaded, it returns an error so `main.rs` can panic
//!    immediately (fail-fast pattern — a cache that can't embed is useless).
//!  - `embed()` is `async` but internally calls `spawn_blocking` because
//!    `fastembed` inference is CPU-bound synchronous work. This prevents
//!    blocking the Tokio executor under concurrent gRPC load.

use std::sync::Arc;

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::info;

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum EmbedError {
    #[error("Failed to initialize local ONNX embedding model: {0}")]
    InitFailed(String),

    #[error("Embedding inference failed: {0}")]
    InferenceFailed(String),

    #[error("Embedder task panicked: {0}")]
    TaskPanic(String),
}

// ── Embedder struct ───────────────────────────────────────────────────────────

/// Thread-safe wrapper around [`fastembed::TextEmbedding`].
///
/// Uses `Arc<Mutex<TextEmbedding>>` because `TextEmbedding` is not `Send +
/// Sync` on its own, and multiple gRPC request tasks may call `embed()`
/// concurrently. The Mutex ensures exclusive access during inference.
#[derive(Clone)]
pub struct LocalEmbedder {
    inner: Arc<Mutex<TextEmbedding>>,
}

impl LocalEmbedder {
    /// Initializes the local ONNX embedder using `BGESmallENV15`.
    ///
    /// **This must be called at service startup.** On success, the ONNX model
    /// is fully loaded into memory. On failure, the caller should treat this
    /// as a fatal boot error and terminate the process.
    ///
    /// The model produces **384-dimensional** vectors — identical to the L1
    /// in-memory cache in `redeye_gateway`, which also uses `BGESmallENV15`.
    pub fn new() -> Result<Self, EmbedError> {
        info!("Loading local ONNX embedding model (BGESmallENV15, 384-dim)...");

        let opts = InitOptions::new(EmbeddingModel::BGESmallENV15);

        let model = TextEmbedding::try_new(opts)
            .map_err(|e| EmbedError::InitFailed(e.to_string()))?;

        info!("Local embedder initialized successfully (384-dim).");

        Ok(Self {
            inner: Arc::new(Mutex::new(model)),
        })
    }

    /// Embeds a single text prompt, returning a 384-dimensional `Vec<f32>`.
    ///
    /// Inference is CPU-bound; this method offloads to `spawn_blocking` to
    /// avoid stalling the Tokio async executor during heavy concurrent load.
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        let embedder = Arc::clone(&self.inner);
        let owned_text = text.to_owned();

        let result = tokio::task::spawn_blocking(move || {
            // Block inside the blocking thread — safe to hold the mutex here.
            let mut guard = embedder.blocking_lock();
            guard
                .embed(vec![owned_text.as_str()], None)
                .map_err(|e| EmbedError::InferenceFailed(e.to_string()))
        })
        .await
        .map_err(|e| EmbedError::TaskPanic(e.to_string()))??;

        result
            .into_iter()
            .next()
            .ok_or_else(|| EmbedError::InferenceFailed("Embedder returned empty output".into()))
    }
}
