ALTER TABLE activities
    ADD COLUMN objectType TEXT GENERATED ALWAYS AS (json_extract(json, "$.object.type")) VIRTUAL;

CREATE INDEX idx_activities_by_publishedYearMonthDay ON activities(publishedYearMonthDay);
CREATE INDEX idx_activities_by_publishedYearMonth ON activities(publishedYearMonth);
CREATE INDEX idx_activities_by_publishedYear ON activities(publishedYear);