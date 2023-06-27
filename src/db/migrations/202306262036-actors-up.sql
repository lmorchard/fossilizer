CREATE TABLE actors (
    json TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,    
    id TEXT GENERATED ALWAYS AS (json_extract(json, "$.id")) VIRTUAL UNIQUE,
    type TEXT GENERATED ALWAYS AS (json_extract(json, "$.type")) VIRTUAL
);
CREATE INDEX idx_actors_by_id ON actors(id);
