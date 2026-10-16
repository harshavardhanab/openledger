CREATE TABLE documents_new (id INTEGER PRIMARY KEY, kind TEXT NOT NULL CHECK(kind IN ('sale','purchase','receipt','payment','stock','credit_note','debit_note','customer_refund','supplier_refund')), number TEXT NOT NULL UNIQUE, date TEXT NOT NULL, party_id INTEGER REFERENCES parties(id), invoice_id INTEGER REFERENCES documents(id), journal_id INTEGER UNIQUE REFERENCES journals(id), total_minor INTEGER NOT NULL CHECK(total_minor>0), request_id TEXT NOT NULL UNIQUE, request_payload TEXT NOT NULL, data TEXT NOT NULL);
INSERT INTO documents_new SELECT * FROM documents;
DROP TABLE documents;
ALTER TABLE documents_new RENAME TO documents;
CREATE INDEX documents_party ON documents(party_id,date,id);
CREATE TRIGGER document_no_update BEFORE UPDATE ON documents BEGIN SELECT RAISE(ABORT,'Posted documents are immutable'); END;
CREATE TRIGGER document_no_delete BEFORE DELETE ON documents BEGIN SELECT RAISE(ABORT,'Posted documents are immutable'); END;
INSERT INTO numbering VALUES ('credit_note','CN-',1),('debit_note','DN-',1),('customer_refund','CRF-',1),('supplier_refund','SRF-',1);
PRAGMA user_version=4;
