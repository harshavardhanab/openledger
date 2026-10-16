-- Version 5 distinguishes explicitly classified nil-rated/exempt/non-GST sales.
-- Data remains immutable JSON; bump the database version so older apps fail closed.
BEGIN IMMEDIATE;
PRAGMA user_version = 5;
COMMIT;
