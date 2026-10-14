BEGIN IMMEDIATE;
PRAGMA application_id = 1330399058;
CREATE TABLE company (id INTEGER PRIMARY KEY CHECK(id=1), name TEXT NOT NULL, currency TEXT NOT NULL, fiscal_year_start TEXT NOT NULL, created_at TEXT NOT NULL);
CREATE TABLE accounts (id INTEGER PRIMARY KEY, code TEXT NOT NULL COLLATE NOCASE UNIQUE, name TEXT NOT NULL, kind TEXT NOT NULL CHECK(kind IN ('asset','liability','equity','income','expense')));
CREATE TABLE journals (
 id INTEGER PRIMARY KEY, date TEXT NOT NULL, reference TEXT NOT NULL, description TEXT NOT NULL,
 request_id TEXT NOT NULL UNIQUE, request_payload TEXT NOT NULL, created_at TEXT NOT NULL,
 total_minor INTEGER NOT NULL CHECK(total_minor>0), status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','posted')),
 reversal_of INTEGER UNIQUE REFERENCES journals(id)
);
CREATE TABLE lines (id INTEGER PRIMARY KEY, journal_id INTEGER NOT NULL REFERENCES journals(id), position INTEGER NOT NULL,
 account_id INTEGER NOT NULL REFERENCES accounts(id), side TEXT NOT NULL CHECK(side IN ('debit','credit')),
 amount_minor INTEGER NOT NULL CHECK(amount_minor>0), UNIQUE(journal_id,position));
CREATE INDEX lines_account ON lines(account_id,journal_id);
CREATE INDEX journals_date ON journals(date,id);
CREATE TRIGGER balanced_posting BEFORE UPDATE OF status ON journals WHEN NEW.status='posted' AND OLD.status='draft' BEGIN
 SELECT CASE WHEN (SELECT COUNT(*) FROM lines WHERE journal_id=NEW.id)<2 THEN RAISE(ABORT,'A journal requires at least two lines') END;
 SELECT CASE WHEN (SELECT COALESCE(SUM(amount_minor),0) FROM lines WHERE journal_id=NEW.id AND side='debit')!=NEW.total_minor THEN RAISE(ABORT,'Debit total does not match journal') END;
 SELECT CASE WHEN (SELECT COALESCE(SUM(amount_minor),0) FROM lines WHERE journal_id=NEW.id AND side='credit')!=NEW.total_minor THEN RAISE(ABORT,'Credit total does not match journal') END;
END;
CREATE TRIGGER journal_immutable_update BEFORE UPDATE ON journals WHEN OLD.status='posted' BEGIN SELECT RAISE(ABORT,'Posted journals are immutable'); END;
CREATE TRIGGER journal_immutable_delete BEFORE DELETE ON journals WHEN OLD.status='posted' BEGIN SELECT RAISE(ABORT,'Posted journals are immutable'); END;
CREATE TRIGGER line_immutable_insert BEFORE INSERT ON lines WHEN (SELECT status FROM journals WHERE id=NEW.journal_id)='posted' BEGIN SELECT RAISE(ABORT,'Posted lines are immutable'); END;
CREATE TRIGGER line_immutable_update BEFORE UPDATE ON lines WHEN (SELECT status FROM journals WHERE id=OLD.journal_id)='posted' OR (SELECT status FROM journals WHERE id=NEW.journal_id)='posted' BEGIN SELECT RAISE(ABORT,'Posted lines are immutable'); END;
CREATE TRIGGER line_immutable_delete BEFORE DELETE ON lines WHEN (SELECT status FROM journals WHERE id=OLD.journal_id)='posted' BEGIN SELECT RAISE(ABORT,'Posted lines are immutable'); END;
CREATE TRIGGER account_immutable_update BEFORE UPDATE ON accounts WHEN EXISTS(SELECT 1 FROM lines JOIN journals ON journals.id=lines.journal_id WHERE lines.account_id=OLD.id AND journals.status='posted') BEGIN SELECT RAISE(ABORT,'Posted account definitions are immutable'); END;
CREATE TRIGGER company_immutable_currency BEFORE UPDATE OF currency ON company WHEN EXISTS(SELECT 1 FROM journals WHERE status='posted') BEGIN SELECT RAISE(ABORT,'Currency cannot change after posting'); END;
PRAGMA user_version = 1;
COMMIT;
