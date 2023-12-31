CREATE INDEX idx_activities_by_publishedYearMonthDay_2 ON activities(publishedYearMonthDay, isPublic);
CREATE INDEX idx_activities_by_publishedYearMonth_2 ON activities(publishedYearMonth, isPublic);
CREATE INDEX idx_activities_by_publishedYear_2 ON activities(publishedYear, isPublic);

DROP INDEX idx_activities_by_publishedYearMonthDay;
DROP INDEX idx_activities_by_publishedYearMonth;
DROP INDEX idx_activities_by_publishedYear;
