CREATE TABLE new_activities (
  schema TEXT DEFAULT "fossilizer::activitystreams::Activity",
  json TEXT,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  -- try to munge together an id column from Mastodon status properties which matches activitypub export
  id TEXT GENERATED ALWAYS AS (
    iif(
      schema == "megalodon::entities::Status",
      iif(
        json_extract(json, '$.reblog') is not null,
        json_extract(json, '$.uri'),
        json_extract(json, '$.uri') || "/activity"
      ),
      json_extract(json, "$.id")
    )
  ) VIRTUAL UNIQUE,
  objectType TEXT GENERATED ALWAYS AS (
    json_extract(json, '$.object.type')
  ) VIRTUAL,
  isPublic INTEGER GENERATED ALWAYS AS (
    json_extract(json, "$.visibility") == 'public'
    or like(
      '%https://www.w3.org/ns/activitystreams#Public%',
      json_extract(json, '$.to')
    )
    or like(
      '%https://www.w3.org/ns/activitystreams#Public%',
      json_extract(json, '$.cc')
    )
  ) VIRTUAL,
  published DATETIME GENERATED ALWAYS AS (
    coalesce(
      json_extract(json, "$.published"),
      json_extract(json, "$.created_at")
    )
  ) VIRTUAL,
  publishedYear DATETIME GENERATED ALWAYS AS (
    strftime(
      "%Y",
      coalesce(
        json_extract(json, "$.published"),
        json_extract(json, "$.created_at")
      )
    )
  ) VIRTUAL,
  publishedYearMonth DATETIME GENERATED ALWAYS AS (
    strftime(
      "%Y/%m",
      coalesce(
        json_extract(json, "$.published"),
        json_extract(json, "$.created_at")
      )
    )
  ) VIRTUAL,
  publishedYearMonthDay DATETIME GENERATED ALWAYS AS (
    strftime(
      "%Y/%m/%d",
      coalesce(
        json_extract(json, "$.published"),
        json_extract(json, "$.created_at")
      )
    )
  ) VIRTUAL
);

INSERT OR REPLACE INTO new_activities(schema, json, created_at)
  SELECT schema, json, created_at FROM activities;

DROP INDEX IF EXISTS idx_activities_by_ispublic;
DROP INDEX IF EXISTS idx_activities_by_publishedYearMonthDay;
DROP INDEX IF EXISTS idx_activities_by_publishedYearMonth;
DROP INDEX IF EXISTS idx_activities_by_publishedYear;

ALTER TABLE activities RENAME TO old_activities;
ALTER TABLE new_activities RENAME TO activities;

CREATE INDEX idx_activities_by_ispublic ON activities(isPublic);
CREATE INDEX idx_activities_by_publishedYearMonthDay ON activities(publishedYearMonthDay, isPublic);
CREATE INDEX idx_activities_by_publishedYearMonth ON activities(publishedYearMonth, isPublic);
CREATE INDEX idx_activities_by_publishedYear ON activities(publishedYear, isPublic);

-- DROP TABLE old_activities;