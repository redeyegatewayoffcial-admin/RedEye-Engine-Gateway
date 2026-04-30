CREATE TABLE IF NOT EXISTS llm_models (
    id UUID PRIMARY KEY,
    model_name VARCHAR UNIQUE NOT NULL,
    provider_name VARCHAR NOT NULL,
    base_url VARCHAR NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
