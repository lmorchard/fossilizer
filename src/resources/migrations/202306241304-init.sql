CREATE TABLE activities (
    json TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    id TEXT GENERATED ALWAYS AS (json_extract(json, "$.id")) VIRTUAL UNIQUE,
    type TEXT GENERATED ALWAYS AS (json_extract(json, "$.type")) VIRTUAL,
    url TEXT GENERATED ALWAYS AS (json_extract(json, "$.object.url")) VIRTUAL,
    summary TEXT GENERATED ALWAYS AS (json_extract(json, "$.object.summary")) VIRTUAL,
    content TEXT GENERATED ALWAYS AS (json_extract(json, "$.object.content")) VIRTUAL,
    published DATETIME GENERATED ALWAYS AS (json_extract(json, "$.object.published")) VIRTUAL,
    actorUrl TEXT GENERATED ALWAYS AS (json_extract(json, "$.actor.url")) VIRTUAL,
    accountAvatarUrl TEXT GENERATED ALWAYS AS (json_extract(json, "$.actor.icon.url")) VIRTUAL,
    accountName TEXT GENERATED ALWAYS AS (json_extract(json, "$.actor.name")) VIRTUAL
)
