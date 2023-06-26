CREATE TABLE activities (
    json TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    id TEXT GENERATED ALWAYS AS (json_extract(json, "$.id")) VIRTUAL UNIQUE,
    type TEXT GENERATED ALWAYS AS (json_extract(json, "$.type")) VIRTUAL,
    url TEXT GENERATED ALWAYS AS (json_extract(json, "$.object.url")) VIRTUAL,
    actorId TEXT GENERATED ALWAYS AS (json_extract(json, "$.actor")) VIRTUAL,
    summary TEXT GENERATED ALWAYS AS (json_extract(json, "$.object.summary")) VIRTUAL,
    content TEXT GENERATED ALWAYS AS (json_extract(json, "$.object.content")) VIRTUAL,
    published DATETIME GENERATED ALWAYS AS (json_extract(json, "$.published")) VIRTUAL,
    publishedYear TEXT GENERATED ALWAYS AS (strftime("%Y", json_extract(json, "$.published"))) VIRTUAL,
    publishedYearMonth TEXT GENERATED ALWAYS AS (strftime("%Y/%m", json_extract(json, "$.published"))) VIRTUAL,
    publishedYearMonthDay TEXT GENERATED ALWAYS AS (strftime("%Y/%m/%d", json_extract(json, "$.published"))) VIRTUAL
)
