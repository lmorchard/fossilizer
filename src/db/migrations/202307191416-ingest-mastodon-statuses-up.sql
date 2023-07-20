-- drop generated columns not directly involved in lookup indexes, we can always re-add later
ALTER TABLE activities DROP COLUMN type;
ALTER TABLE activities DROP COLUMN url;
ALTER TABLE activities DROP COLUMN actorId;
ALTER TABLE activities DROP COLUMN summary;
ALTER TABLE activities DROP COLUMN content;
-- drop indexes, because we're going to redefine the dependent columns
DROP INDEX idx_activities_by_ispublic;
DROP INDEX idx_activities_by_publishedYearMonthDay_2;
DROP INDEX idx_activities_by_publishedYearMonth_2;
DROP INDEX idx_activities_by_publishedYear_2;
-- drop the columns for published time, since we're going to redefine these
ALTER TABLE activities DROP COLUMN isPublic;
ALTER TABLE activities DROP COLUMN published;
ALTER TABLE activities DROP COLUMN publishedYear;
ALTER TABLE activities DROP COLUMN publishedYearMonth;
ALTER TABLE activities DROP COLUMN publishedYearMonthDay;
-- add revised generated columns to accomodate mastodon status JSON
ALTER TABLE activities
ADD COLUMN isPublic INTEGER GENERATED ALWAYS AS (
        json_extract(json, "$.visibility") == 'public'
        or like(
            '%https://www.w3.org/ns/activitystreams#Public%',
            json_extract(json, '$.to')
        )
        or like(
            '%https://www.w3.org/ns/activitystreams#Public%',
            json_extract(json, '$.cc')
        )
    ) VIRTUAL;
ALTER TABLE activities
ADD COLUMN published DATETIME GENERATED ALWAYS AS (
        coalesce(
            json_extract(json, "$.published"),
            json_extract(json, "$.created_at")
        )
    ) VIRTUAL;
ALTER TABLE activities
ADD COLUMN publishedYear DATETIME GENERATED ALWAYS AS (
        strftime(
            "%Y",
            coalesce(
                json_extract(json, "$.published"),
                json_extract(json, "$.created_at")
            )
        )
    ) VIRTUAL;
ALTER TABLE activities
ADD COLUMN publishedYearMonth DATETIME GENERATED ALWAYS AS (
        strftime(
            "%Y/%m",
            coalesce(
                json_extract(json, "$.published"),
                json_extract(json, "$.created_at")
            )
        )
    ) VIRTUAL;
ALTER TABLE activities
ADD COLUMN publishedYearMonthDay DATETIME GENERATED ALWAYS AS (
        strftime(
            "%Y/%m/%d",
            coalesce(
                json_extract(json, "$.published"),
                json_extract(json, "$.created_at")
            )
        )
    ) VIRTUAL;
-- all JSON added until now should be an activitystreams Activity
ALTER TABLE activities
ADD COLUMN schema TEXT DEFAULT "fossilizer::activitystreams::Activity";
-- re-add the lookup indexes
CREATE INDEX idx_activities_by_ispublic ON activities(isPublic);
CREATE INDEX idx_activities_by_publishedYearMonthDay ON activities(publishedYearMonthDay, isPublic);
CREATE INDEX idx_activities_by_publishedYearMonth ON activities(publishedYearMonth, isPublic);
CREATE INDEX idx_activities_by_publishedYear ON activities(publishedYear, isPublic);