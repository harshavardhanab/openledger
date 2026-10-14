BEGIN IMMEDIATE;
CREATE TABLE gst_profile (id INTEGER PRIMARY KEY CHECK(id=1), data TEXT NOT NULL);
CREATE TRIGGER gst_profile_no_update BEFORE UPDATE ON gst_profile BEGIN SELECT RAISE(ABORT,'GST registration mappings are immutable'); END;
CREATE TRIGGER gst_profile_no_delete BEFORE DELETE ON gst_profile BEGIN SELECT RAISE(ABORT,'GST registration mappings are immutable'); END;
PRAGMA user_version=3;
COMMIT;
