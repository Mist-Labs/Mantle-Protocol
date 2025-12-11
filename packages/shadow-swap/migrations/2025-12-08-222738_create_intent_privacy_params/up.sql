-- Your SQL goes here
CREATE TABLE intent_privacy_params (
    intent_id TEXT PRIMARY KEY REFERENCES intents(id) ON DELETE CASCADE,
    commitment TEXT,
    nullifier TEXT,
    secret TEXT,
    recipient TEXT,
    claim_signature TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_privacy_nullifier ON intent_privacy_params(nullifier);
CREATE INDEX idx_privacy_commitment ON intent_privacy_params(commitment);
