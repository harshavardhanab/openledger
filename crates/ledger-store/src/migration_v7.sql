BEGIN IMMEDIATE;
CREATE TABLE itc_adjustments (
 id INTEGER PRIMARY KEY,
 invoice_id INTEGER NOT NULL REFERENCES documents(id),
 deferral_id INTEGER REFERENCES itc_adjustments(id),
 pending_account_id INTEGER NOT NULL REFERENCES accounts(id),
 journal_id INTEGER NOT NULL UNIQUE REFERENCES journals(id),
 request_id TEXT NOT NULL UNIQUE,
 data TEXT NOT NULL
);
CREATE INDEX itc_adjustments_invoice ON itc_adjustments(invoice_id);
CREATE TRIGGER itc_no_update BEFORE UPDATE ON itc_adjustments BEGIN SELECT RAISE(ABORT,'ITC adjustments are immutable'); END;
CREATE TRIGGER itc_no_delete BEFORE DELETE ON itc_adjustments BEGIN SELECT RAISE(ABORT,'ITC adjustments are immutable'); END;
PRAGMA user_version=7;
COMMIT;
