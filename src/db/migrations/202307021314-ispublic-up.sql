ALTER TABLE activities
    ADD COLUMN isPublic INTEGER
    GENERATED ALWAYS AS 
    (
      like("%https://www.w3.org/ns/activitystreams#Public%", json_extract(json, "$.to")) or
      like("%https://www.w3.org/ns/activitystreams#Public%", json_extract(json, "$.cc"))
    )
    VIRTUAL;

CREATE INDEX idx_activities_by_ispublic ON activities(isPublic);
