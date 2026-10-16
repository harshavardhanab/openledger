import { useEffect, useId, useRef, useState } from 'react';
import { Snapshot, download, money, request, today } from './api';
type Amounts = {
  taxable_minor: string;
  cgst_minor: string;
  sgst_minor: string;
  igst_minor: string;
};
type Draft = {
  format: string;
  version: number;
  from: string;
  to: string;
  gstin: string;
  filing_ready: false;
  portal_verified: false;
  query: { period: string; frequency: string };
  issues: { code: string; reference: string; message: string; document_id: number | null }[];
  gstr1_sections: Record<string, Amounts>;
  gstr1_rows: { section: string }[];
  gstr3b_book_3_1_a: Amounts;
  gstr3b_book_3_1_c: Amounts;
  gstr3b_book_3_1_e: Amounts;
  gstr3b_draft_4_a_5: Amounts;
  gstr3b_draft_4_b_1: Amounts;
  gstr3b_draft_4_b_2: Amounts;
  gstr3b_draft_4_d_1: Amounts;
  gstr3b_draft_4_c: Amounts;
  itc_adjustments: {
    id: number;
    input: {
      date: string;
      invoice_id: number;
      deferral_id: number | null;
      reason: string;
      cgst: string;
      sgst: string;
      igst: string;
    };
  }[];
  gstr3b_draft_4_d_2: Amounts;
  book_recoverable_input_tax: Amounts;
  input_tax_rows: {
    number: string;
    date: string;
    event: string;
    invoice_tax: Amounts;
    recoverable_tax: Amounts;
    nonrecoverable_tax: Amounts;
    allocation?: { return_treatment?: string; reason: string } | null;
  }[];
  gstr3b_book_5: { category: string; inter_state_minor: string; intra_state_minor: string }[];
  gstr3b_book_3_2_unregistered: Record<string, Amounts>;
  reconciliation: {
    direction: string;
    component: string;
    book_activity_minor: string;
    register_activity_minor: string;
    difference_minor: string;
  }[];
  reconciled: boolean;
  files: { section: string; filename: string; row_count: number; contents: string }[];
  review_requirements: string[];
};
export function GstReturns({ state }: { state: Snapshot }) {
  const id = useId();
  const generation = useRef(0);
  const [period, setPeriod] = useState(today().slice(0, 7));
  const [frequency, setFrequency] = useState('monthly');
  const [draft, setDraft] = useState<Draft | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [issuePage, setIssuePage] = useState(0);
  const [inputPage, setInputPage] = useState(0);
  // A report must never survive a change of books, dates or accounting data.
  useEffect(() => {
    generation.current++;
    setBusy(false);
    setDraft(null);
    setError('');
    setIssuePage(0);
    setInputPage(0);
    return () => {
      generation.current++;
    };
  }, [state, period, frequency]);
  return (
    <section className="panel business-card" aria-labelledby={`${id}-title`}>
      <h3 id={`${id}-title`}>GST return preparation</h3>
      <p className="form-hint">
        Prepare draft GSTR-1 CSVs and a GSTR-3B book worksheet for supported domestic supplies.
        Review all applicable tables and GSTR-2B/IMS before filing. These files have not been
        accepted by the portal.
      </p>
      <form
        onSubmit={async (e) => {
          e.preventDefault();
          setBusy(true);
          setError('');
          setDraft(null);
          setIssuePage(0);
          const current = ++generation.current;
          try {
            const result = await request<Draft>('gst_return_draft', { period, frequency });
            if (current === generation.current) setDraft(result);
          } catch (e) {
            if (current === generation.current)
              setError(e instanceof Error ? e.message : String(e));
          } finally {
            if (current === generation.current) setBusy(false);
          }
        }}
      >
        <div className="business-grid">
          <div className="business-field">
            <label htmlFor={`${id}-period`}>GST return month / quarter end</label>
            <input
              id={`${id}-period`}
              type="month"
              min="2017-07"
              required
              value={period}
              disabled={busy}
              onChange={(e) => setPeriod(e.target.value)}
            />
          </div>
          <div className="business-field">
            <label htmlFor={`${id}-frequency`}>GST return frequency</label>
            <select
              id={`${id}-frequency`}
              value={frequency}
              disabled={busy}
              onChange={(e) => setFrequency(e.target.value)}
            >
              <option value="monthly">Monthly</option>
              <option value="quarterly">Quarterly (quarter-ending month)</option>
            </select>
          </div>
        </div>
        <div className="business-actions">
          <button className="button secondary" disabled={busy}>
            {busy ? 'Preparing draft…' : 'Prepare GST return draft'}
          </button>
        </div>
      </form>
      {error && <p role="alert">{error}</p>}
      {draft && (
        <>
          <p role="status">
            Draft for {draft.from} through {draft.to} · {draft.gstin}.{' '}
            {draft.reconciled
              ? 'GST control activity matches the register.'
              : 'GST control activity differs from the register.'}{' '}
            {draft.issues.length} review items.
          </p>
          <h4>GSTR-1 candidate sections</h4>
          <AmountTable
            rows={Object.entries(draft.gstr1_sections).map(([label, a]) => ({
              label: label.toUpperCase(),
              ...a,
            }))}
          />
          {draft.itc_adjustments.length > 0 && (
            <p>
              {draft.itc_adjustments.length} linked temporary ITC reversal/reclaim events are
              included in this draft. Review their full source details in the complete JSON and
              purchase details.
            </p>
          )}
          <h4>GSTR-3B book worksheet</h4>
          <AmountTable
            rows={[
              { label: '3.1(a) · Outward taxable supplies', ...draft.gstr3b_book_3_1_a },
              { label: '3.1(c) · Nil-rated / exempt outward supplies', ...draft.gstr3b_book_3_1_c },
              { label: '3.1(e) · Non-GST outward supplies', ...draft.gstr3b_book_3_1_e },
              {
                label: '4(A)(5) · Draft other ITC before GSTR-2B/IMS review',
                ...draft.gstr3b_draft_4_a_5,
                taxable_minor: '',
              },
              {
                label: '4(B)(1) · Draft reviewed permanent reversals',
                ...draft.gstr3b_draft_4_b_1,
                taxable_minor: '',
              },
              {
                label: '4(B)(2) · Draft reviewed temporary reversals',
                ...draft.gstr3b_draft_4_b_2,
                taxable_minor: '',
              },
              {
                label: '4(D)(1) · Draft linked reclaims included in 4(A)(5)',
                ...draft.gstr3b_draft_4_d_1,
                taxable_minor: '',
              },
              {
                label: '4(C) · Draft net mapped ITC',
                ...draft.gstr3b_draft_4_c,
                taxable_minor: '',
              },
              {
                label: '4(D)(2) · Draft reviewed excluded ineligible tax',
                ...draft.gstr3b_draft_4_d_2,
                taxable_minor: '',
              },
              {
                label: 'Recoverable input tax in books',
                ...draft.book_recoverable_input_tax,
                taxable_minor: '',
              },
              ...Object.entries(draft.gstr3b_book_3_2_unregistered).map(([state, a]) => ({
                label: `3.2 · Unregistered · POS ${state}`,
                ...a,
              })),
            ]}
          />
          <h4>GSTR-3B table 5 · Classified inward supplies</h4>
          <div className="table-scroll">
            <table>
              <thead>
                <tr>
                  <th>Category</th>
                  <th>Inter-state</th>
                  <th>Intra-state</th>
                </tr>
              </thead>
              <tbody>
                {draft.gstr3b_book_5.map((r) => (
                  <tr key={r.category}>
                    <td>{r.category}</td>
                    <td>{money(r.inter_state_minor, 'INR')}</td>
                    <td>{money(r.intra_state_minor, 'INR')}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          {draft.input_tax_rows.length > 0 && (
            <details>
              <summary>Input tax source details ({draft.input_tax_rows.length})</summary>
              <div className="table-scroll">
                <table>
                  <thead>
                    <tr>
                      <th>Document / event</th>
                      <th>Invoice tax</th>
                      <th>Recoverable</th>
                      <th>Nonrecoverable</th>
                      <th>Reviewed mapping / basis</th>
                    </tr>
                  </thead>
                  <tbody>
                    {draft.input_tax_rows
                      .slice(inputPage * 50, inputPage * 50 + 50)
                      .map((r, index) => {
                        const total = (a: Amounts) =>
                          BigInt(a.cgst_minor) + BigInt(a.sgst_minor) + BigInt(a.igst_minor);
                        return (
                          <tr key={`${r.number}-${r.event}-${index}`}>
                            <td>
                              {r.number} · {r.date} · {r.event.replaceAll('_', ' ')}
                            </td>
                            <td>{money(total(r.invoice_tax), 'INR')}</td>
                            <td>{money(total(r.recoverable_tax), 'INR')}</td>
                            <td>{money(total(r.nonrecoverable_tax), 'INR')}</td>
                            <td>
                              {r.allocation
                                ? `${r.allocation.return_treatment?.replaceAll('_', ' ') || 'Mapping pending'} · ${r.allocation.reason}`
                                : 'Reviewed recoverable purchase'}
                            </td>
                          </tr>
                        );
                      })}
                  </tbody>
                </table>
              </div>
              <div className="business-actions">
                <button
                  className="button secondary"
                  disabled={inputPage === 0}
                  onClick={() => setInputPage((n) => n - 1)}
                >
                  Previous input tax rows
                </button>
                <span>
                  {inputPage * 50 + 1}–{Math.min(inputPage * 50 + 50, draft.input_tax_rows.length)}{' '}
                  of {draft.input_tax_rows.length}
                </span>
                <button
                  className="button secondary"
                  disabled={(inputPage + 1) * 50 >= draft.input_tax_rows.length}
                  onClick={() => setInputPage((n) => n + 1)}
                >
                  Next input tax rows
                </button>
              </div>
            </details>
          )}
          <h4>GST control reconciliation</h4>
          <div className="table-scroll">
            <table>
              <thead>
                <tr>
                  <th>Control</th>
                  <th>Book activity</th>
                  <th>Register activity</th>
                  <th>Difference</th>
                </tr>
              </thead>
              <tbody>
                {draft.reconciliation.map((r) => (
                  <tr key={`${r.direction}-${r.component}`}>
                    <td>
                      {r.direction} {r.component}
                    </td>
                    <td>{money(r.book_activity_minor, 'INR')}</td>
                    <td>{money(r.register_activity_minor, 'INR')}</td>
                    <td>{money(r.difference_minor, 'INR')}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <h4>Review before filing</h4>
          <ul>
            {draft.review_requirements.map((s) => (
              <li key={s}>{s}</li>
            ))}
          </ul>
          {draft.issues.length > 0 && (
            <>
              <div className="table-scroll">
                <table>
                  <thead>
                    <tr>
                      <th>Reference</th>
                      <th>Review item</th>
                    </tr>
                  </thead>
                  <tbody>
                    {draft.issues.slice(issuePage * 50, issuePage * 50 + 50).map((i, index) => (
                      <tr key={`${i.code}-${i.document_id}-${index}`}>
                        <td>{i.reference}</td>
                        <td>{i.message}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              <div className="business-actions">
                <button
                  className="button secondary"
                  disabled={issuePage === 0}
                  onClick={() => setIssuePage((n) => n - 1)}
                >
                  Previous review items
                </button>
                <span>
                  {issuePage * 50 + 1}–{Math.min(issuePage * 50 + 50, draft.issues.length)} of{' '}
                  {draft.issues.length}
                </span>
                <button
                  className="button secondary"
                  disabled={(issuePage + 1) * 50 >= draft.issues.length}
                  onClick={() => setIssuePage((n) => n + 1)}
                >
                  Next review items
                </button>
              </div>
            </>
          )}
          <div className="business-actions">
            <button
              className="button secondary"
              onClick={() =>
                void download(
                  `DRAFT-gst-return-${draft.query.period}.json`,
                  JSON.stringify(draft, null, 2),
                )
              }
            >
              Export complete GST draft (JSON)
            </button>
            {draft.files.map((f) => (
              <button
                className="button secondary"
                key={f.section}
                disabled={f.row_count === 0}
                onClick={() => void download(f.filename, f.contents, 'text/csv')}
              >
                Export draft{' '}
                {f.section === 'gstr3b-book-summary'
                  ? 'GSTR-3B book worksheet'
                  : f.section.toUpperCase()}{' '}
                ({f.row_count})
              </button>
            ))}
          </div>
        </>
      )}
    </section>
  );
}
function AmountTable({ rows }: { rows: (Amounts & { label: string })[] }) {
  return (
    <div className="table-scroll">
      <table>
        <thead>
          <tr>
            <th>Section</th>
            <th>Supply / taxable value</th>
            <th>CGST</th>
            <th>SGST / UTGST</th>
            <th>IGST</th>
          </tr>
        </thead>
        <tbody>
          {rows.length ? (
            rows.map((r) => (
              <tr key={r.label}>
                <td>{r.label}</td>
                <td>{r.taxable_minor ? money(r.taxable_minor, 'INR') : '—'}</td>
                <td>{money(r.cgst_minor, 'INR')}</td>
                <td>{money(r.sgst_minor, 'INR')}</td>
                <td>{money(r.igst_minor, 'INR')}</td>
              </tr>
            ))
          ) : (
            <tr>
              <td colSpan={5}>No candidate outward supplies in this period.</td>
            </tr>
          )}
        </tbody>
      </table>
    </div>
  );
}
