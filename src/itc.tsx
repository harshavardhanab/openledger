import { useEffect, useId, useRef, useState } from 'react';
import { Snapshot, decimal, download, money, request, today } from './api';
type Input = {
  invoice_id: number;
  deferral_id: number | null;
  pending_account_id: number;
  date: string;
  cgst: string;
  sgst: string;
  igst: string;
  reason: string;
  eligibility_reviewed: boolean;
  request_id: string;
};
type Record = {
  id: number;
  journal_id: number;
  input: Input;
  remaining: { cgst_minor: string; sgst_minor: string; igst_minor: string };
};
type Preview = { lines: { account_id: number; side: string; amount_minor: string }[] };
export function ItcAdjustments({
  invoice,
  state,
  busy,
  mutate,
}: {
  invoice: { id: number; number: string };
  state: Snapshot;
  busy: boolean;
  mutate: (command: string, payload: object, message: string) => Promise<boolean>;
}) {
  const id = useId();
  const generation = useRef(0);
  const fresh = (): Input => ({
    invoice_id: invoice.id,
    deferral_id: null,
    pending_account_id: 0,
    date: today(),
    cgst: '0',
    sgst: '0',
    igst: '0',
    reason: '',
    eligibility_reviewed: false,
    request_id: crypto.randomUUID(),
  });
  const [input, setInput] = useState<Input>(fresh);
  const [records, setRecords] = useState<Record[]>([]);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [working, setWorking] = useState(true);
  const [error, setError] = useState('');
  const [page, setPage] = useState(0);
  useEffect(() => {
    const current = ++generation.current;
    setInput(fresh());
    setPreview(null);
    setRecords([]);
    setPage(0);
    setWorking(true);
    setError('');
    void request<Record[]>('itc_adjustments', { invoice_id: invoice.id })
      .then((rows) => {
        if (current === generation.current) {
          setRecords(rows);
          setWorking(false);
        }
      })
      .catch((e) => {
        if (current === generation.current) {
          setError(String(e));
          setWorking(false);
        }
      });
    return () => {
      generation.current++;
    };
    // Reset every editable value when the books or posted state changes.
  }, [state, invoice.id]);
  const change = (patch: Partial<Input>) => {
    generation.current++;
    setInput((old) => ({ ...old, ...patch }));
    setPreview(null);
    setError('');
  };
  const disabled = busy || working;
  const currency = state.company!.currency;
  return (
    <section aria-labelledby={`${id}-title`}>
      <h3 id={`${id}-title`}>Temporary ITC reversals & reclaims</h3>
      <p className="form-hint">
        Move reviewed recoverable tax to a dedicated pending ITC asset account, then reclaim it
        against the original reversal after reviewing the supporting evidence. This records your
        eligibility decision. Resolve pending amounts before a supplier return. Interest and portal
        filing require separate review.
      </p>
      <form
        onSubmit={async (e) => {
          e.preventDefault();
          setWorking(true);
          setError('');
          setPreview(null);
          const current = ++generation.current;
          try {
            const result = await request<Preview>('preview_itc_adjustment', input);
            if (current === generation.current) setPreview(result);
          } catch (e) {
            if (current === generation.current) setError(String(e));
          } finally {
            if (current === generation.current) setWorking(false);
          }
        }}
      >
        <div className="business-grid">
          <div className="business-field">
            <label htmlFor={`${id}-action`}>ITC adjustment</label>
            <select
              id={`${id}-action`}
              disabled={disabled}
              value={input.deferral_id ?? ''}
              onChange={(e) => {
                const source = records.find((r) => r.id === Number(e.target.value));
                change(
                  source
                    ? {
                        deferral_id: source.id,
                        pending_account_id: source.input.pending_account_id,
                        cgst: decimal(BigInt(source.remaining.cgst_minor)),
                        sgst: decimal(BigInt(source.remaining.sgst_minor)),
                        igst: decimal(BigInt(source.remaining.igst_minor)),
                        eligibility_reviewed: false,
                        reason: '',
                        request_id: crypto.randomUUID(),
                      }
                    : { ...fresh() },
                );
              }}
            >
              <option value="">New temporary reversal</option>
              {records
                .filter(
                  (r) =>
                    r.input.deferral_id === null &&
                    [r.remaining.cgst_minor, r.remaining.sgst_minor, r.remaining.igst_minor].some(
                      (n) => BigInt(n) > 0n,
                    ),
                )
                .map((r) => (
                  <option key={r.id} value={r.id}>
                    Reclaim reversal {r.id} · {r.input.date}
                  </option>
                ))}
            </select>
          </div>
          <div className="business-field">
            <label htmlFor={`${id}-account`}>Pending ITC asset account</label>
            <select
              id={`${id}-account`}
              required
              disabled={disabled || input.deferral_id !== null}
              value={input.pending_account_id || ''}
              onChange={(e) => change({ pending_account_id: Number(e.target.value) })}
            >
              <option value="">Select dedicated asset account</option>
              {state.accounts
                .filter((a) => a.kind === 'asset')
                .map((a) => (
                  <option key={a.id} value={a.id}>
                    {a.code} · {a.name}
                  </option>
                ))}
            </select>
          </div>
          <div className="business-field">
            <label htmlFor={`${id}-date`}>ITC effective date</label>
            <input
              id={`${id}-date`}
              type="date"
              required
              disabled={disabled}
              value={input.date}
              onChange={(e) => change({ date: e.target.value })}
            />
          </div>
          {(['cgst', 'sgst', 'igst'] as const).map((key) => (
            <div className="business-field" key={key}>
              <label htmlFor={`${id}-${key}`}>
                {key === 'sgst' ? 'SGST / UTGST' : key.toUpperCase()} adjustment amount
              </label>
              <input
                id={`${id}-${key}`}
                inputMode="decimal"
                pattern="[0-9]+(\.[0-9]{1,2})?"
                required
                disabled={disabled}
                value={input[key]}
                onChange={(e) => change({ [key]: e.target.value })}
              />
            </div>
          ))}
          <div className="business-field">
            <label htmlFor={`${id}-reason`}>ITC review basis</label>
            <textarea
              id={`${id}-reason`}
              required
              maxLength={500}
              disabled={disabled}
              value={input.reason}
              onChange={(e) => change({ reason: e.target.value })}
            />
          </div>
        </div>
        <label className="checkbox-label">
          <input
            type="checkbox"
            required
            disabled={disabled}
            checked={input.eligibility_reviewed}
            onChange={(e) => change({ eligibility_reviewed: e.target.checked })}
          />
          I reviewed the temporary reversal or reclaim conditions and supporting evidence.
        </label>
        <div className="business-actions">
          <button className="button secondary" disabled={disabled}>
            Preview ITC posting
          </button>
        </div>
      </form>
      {error && <p role="alert">{error}</p>}
      {preview && (
        <div>
          <h4>Review ITC posting</h4>
          <ul>
            {preview.lines.map((line, index) => (
              <li key={index}>
                {line.side} · {state.accounts.find((a) => a.id === line.account_id)?.name} ·{' '}
                {money(line.amount_minor, currency)}
              </li>
            ))}
          </ul>
          <button
            className="button primary"
            disabled={disabled}
            onClick={async () => {
              if (await mutate('post_itc_adjustment', input, 'ITC adjustment posted.'))
                setPreview(null);
            }}
          >
            Post reviewed ITC adjustment
          </button>
        </div>
      )}
      <h4>Linked ITC history ({records.length})</h4>
      <div className="business-actions">
        <button
          className="button secondary"
          disabled={!records.length}
          onClick={() =>
            void download(
              `itc-${invoice.number}.json`,
              JSON.stringify(
                { invoice_id: invoice.id, invoice_number: invoice.number, adjustments: records },
                null,
                2,
              ),
            )
          }
        >
          Export ITC history
        </button>
      </div>
      <div className="table-scroll">
        <table>
          <thead>
            <tr>
              <th>ID / date</th>
              <th>Action</th>
              <th>CGST / SGST / IGST</th>
              <th>Remaining pending CGST / SGST / IGST</th>
              <th>Review basis</th>
            </tr>
          </thead>
          <tbody>
            {[...records]
              .reverse()
              .slice(page * 50, (page + 1) * 50)
              .map((r) => (
                <tr key={r.id}>
                  <td>
                    {r.id} · {r.input.date}
                  </td>
                  <td>
                    {r.input.deferral_id === null
                      ? 'Temporary reversal'
                      : `Reclaim of ${r.input.deferral_id}`}
                  </td>
                  <td>
                    {r.input.cgst} / {r.input.sgst} / {r.input.igst}
                  </td>
                  <td>
                    {[r.remaining.cgst_minor, r.remaining.sgst_minor, r.remaining.igst_minor]
                      .map((n) => money(n, currency))
                      .join(' / ')}
                  </td>
                  <td>{r.input.reason}</td>
                </tr>
              ))}
          </tbody>
        </table>
      </div>
      {records.length > 50 && (
        <div className="business-actions">
          <button className="button secondary" disabled={!page} onClick={() => setPage(page - 1)}>
            Previous ITC page
          </button>
          <span>Page {page + 1}</span>
          <button
            className="button secondary"
            disabled={(page + 1) * 50 >= records.length}
            onClick={() => setPage(page + 1)}
          >
            Next ITC page
          </button>
        </div>
      )}
    </section>
  );
}
