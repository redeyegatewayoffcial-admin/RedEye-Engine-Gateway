-- ==============================================================================
-- Migration 001: Resize semantic_cache embedding column 1536 → 384 dimensions.
-- ==============================================================================
--
-- CONTEXT:
--   The L2 semantic cache previously used OpenAI text-embedding-3-small
--   (1536-dim). It has been replaced with the local fastembed BGESmallENV15
--   model (384-dim) to match the L1 in-memory cache in redeye_gateway.
--   Vectors from different models are in incompatible latent spaces; no
--   migration of existing data is possible or meaningful.
--
-- EXECUTION ORDER IS CRITICAL:
--   pgvector requires all dependent indexes to be dropped BEFORE an
--   ALTER COLUMN TYPE can succeed. Attempting ALTER without dropping the
--   HNSW index first will result in: ERROR: cannot alter type of a column
--   used by an index or statistics object.

-- STEP 1: Flush all existing 1536-dim vectors.
--   TRUNCATE is used over DELETE for performance (no row-level WAL overhead,
--   instant HNSW index reset, minimal I/O on large tables).
TRUNCATE TABLE semantic_cache;

-- STEP 2: Drop the HNSW index that depends on the embedding column type.
--   Must happen BEFORE ALTER COLUMN or Postgres will reject the type change.
DROP INDEX IF EXISTS idx_semantic_cache_embedding;

-- STEP 3: Resize the embedding column to 384 dimensions.
ALTER TABLE semantic_cache ALTER COLUMN embedding TYPE vector(384);

-- STEP 4: Rebuild the HNSW index for cosine similarity on the new 384-dim space.
--   Parameters are preserved from the original index definition:
--     m = 16             → max neighbours per layer (controls recall vs. memory)
--     ef_construction=64 → build-time search width (controls recall vs. build speed)
CREATE INDEX idx_semantic_cache_embedding
    ON semantic_cache USING hnsw (embedding vector_cosine_ops)
    WITH (m = 16, ef_construction = 64);
