import React, { useEffect, useState, useId } from 'react';
import {
  Account,
  Journal,
  Kind,
  Snapshot,
  csv,
  decimal,
  displayDate,
  download,
  money,
  parseMinor,
  request,
  today,
} from './api';
export type BusinessPage = 'documents' | 'inventory' | 'statements' | 'reconciliation' | 'settings';
type Mutation = (command: string, payload: object, message: string) => Promise<boolean>;
type Party = {
  id: number;
  input: { name: string; kind: string; contact: string; account_id: number };
};
type Item = {
  id: number;
  input: {
    code: string;
    name: string;
    unit: string;
    asset_account_id: number;
    expense_account_id: number;
  };
};
type DocLine = {
  description: string;
  quantity: string;
  unit_price: string;
  account_id: number;
  item_id: number | null;
};
type GstAccounts = { cgst: number; sgst: number; igst: number };
type GstProfile = {
  gstin: string;
  state_code: string;
  state_name: string;
  address: string;
  output: GstAccounts;
  input: GstAccounts;
};
type GstInput = {
  supply_state: string;
  supply_state_name: string;
  counterparty_gstin: string | null;
  counterparty_address: string;
  delivery_address: string;
  supplier_reference: string | null;
  credit_reviewed: boolean;
  rates_bps: number[];
  hsn_sac: string[];
};
type GstCalculation = {
  taxable_minor: string;
  cgst_minor: string;
  sgst_minor: string;
  igst_minor: string;
  lines: {
    taxable_minor: string;
    rate_bps: number;
    hsn_sac: string;
    cgst_minor: string;
    sgst_minor: string;
    igst_minor: string;
  }[];
};
type ComposerLine = DocLine & { gst_rate?: string; hsn_sac?: string };
type DocInput = {
  gst?: GstInput;
  kind: string;
  date: string;
  due_date: string | null;
  party_id: number | null;
  invoice_id: number | null;
  money_account_id: number | null;
  amount: string | null;
  note: string;
  lines: DocLine[];
  request_id: string;
};
type Doc = {
  gst?: GstCalculation;
  id: number;
  number: string;
  journal_id: number;
  total_minor: string;
  input: DocInput;
  paid_minor: string;
  outstanding_minor: string;
  reversed: boolean;
};
type Numbering = { kind: string; prefix: string; next: number };
type Period = {
  id: number;
  start: string;
  end: string;
  journal_id: number | null;
  equity_account_id: number;
};
type Reconciliation = {
  id: number;
  input: {
    account_id: number;
    start: string;
    end: string;
    opening: string;
    closing: string;
    reference: string;
    line_ids: number[];
  };
};
type Business = {
  gst: GstProfile | null;
  parties: Party[];
  items: Item[];
  documents: Doc[];
  stock: { item: Item; quantity_milli: string; value_minor: string }[];
  numbering: Numbering[];
  periods: Period[];
  reconciliations: Reconciliation[];
  document_count: number;
};
type Props = {
  page: BusinessPage;
  state: Snapshot;
  busy: boolean;
  mutate: Mutation;
  onError: (error: string) => void;
  openJournal: (journal: Journal) => void;
};
const docNames: Record<string, string> = {
  sale: 'Sales invoice',
  purchase: 'Purchase invoice',
  receipt: 'Receipt',
  payment: 'Payment',
  stock: 'Stock adjustment',
  close: 'Period close',
};
const err = (e: unknown) => (e instanceof Error ? e.message : String(e));
function Field({ name, children }: { name: string; children: React.ReactNode }) {
  const id = useId();
  return (
    <div className="business-field">
      <label htmlFor={id}>{name}</label>
      {React.cloneElement(children as React.ReactElement<{ id?: string }>, { id })}
    </div>
  );
}
function AccountSelect({
  accounts,
  kinds,
  value,
  change,
  label = 'Account',
}: {
  accounts: Account[];
  kinds: Kind[];
  value: number;
  change: (id: number) => void;
  label?: string;
}) {
  return (
    <Field name={label}>
      <select required value={value || ''} onChange={(e) => change(Number(e.target.value))}>
        <option value="">Select account</option>
        {accounts
          .filter((a) => kinds.includes(a.kind))
          .map((a) => (
            <option key={a.id} value={a.id}>
              {a.code} · {a.name}
            </option>
          ))}
      </select>
    </Field>
  );
}
function quantity(s: string): bigint | null {
  if (!/^(?:\d+(?:\.\d{0,3})?|\.\d{1,3})$/.test(s)) return null;
  const [w, f = ''] = s.split('.');
  return BigInt(w || '0') * 1000n + BigInt(f.padEnd(3, '0'));
}
function lineTotal(line: DocLine): bigint {
  const q = quantity(line.quantity),
    p = parseMinor(line.unit_price);
  return q !== null && p !== null ? (q * p + 500n) / 1000n : 0n;
}
function quantityText(s: string) {
  const n = BigInt(s),
    a = n < 0n ? -n : n;
  return `${n < 0n ? '-' : ''}${a / 1000n}.${String(a % 1000n).padStart(3, '0')}`;
}
export function BusinessWorkspace(props: Props) {
  const [data, setData] = useState<Business | null>(null),
    [offset, setOffset] = useState(0),
    [asOf, setAsOf] = useState('');
  useEffect(() => {
    let live = true;
    void request<Business>('business', { offset, limit: 50, as_of: asOf || null })
      .then((v) => {
        if (live) setData(v);
      })
      .catch((e) => {
        if (live) props.onError(err(e));
      });
    return () => {
      live = false;
    };
  }, [props.state, offset, asOf]);
  if (!data) return <p role="status">Loading business records…</p>;
  return (
    <div className="business-stack">
      {props.page === 'documents' && (
        <Documents
          {...props}
          data={data}
          offset={offset}
          setOffset={setOffset}
          asOf={asOf}
          setAsOf={setAsOf}
        />
      )}
      {props.page === 'inventory' && (
        <Inventory {...props} data={data} asOf={asOf} setAsOf={setAsOf} />
      )}
      {props.page === 'statements' && <Statements {...props} gstConfigured={Boolean(data.gst)} />}
      {props.page === 'reconciliation' && <Bank {...props} data={data} />}
      {props.page === 'settings' && <Settings {...props} data={data} />}
    </div>
  );
}
function Documents(
  props: Props & {
    data: Business;
    offset: number;
    setOffset: (n: number) => void;
    asOf: string;
    setAsOf: (s: string) => void;
  },
) {
  const { data, state, busy, mutate, onError, openJournal } = props;
  const [compose, setCompose] = useState<string | null>(null),
    [settle, setSettle] = useState<Doc | null>(null),
    [detail, setDetail] = useState<Doc | null>(null),
    [showParties, setShowParties] = useState(false),
    [filter, setFilter] = useState('all');
  const currency = state.company!.currency;
  const records = data.documents.filter((d) => filter === 'all' || d.input.kind === filter);
  const open = (id: number) =>
    void request<Journal>('journal', { id })
      .then(openJournal)
      .catch((e) => onError(err(e)));
  return (
    <>
      <div className="panel filter-panel">
        <button
          className="button primary"
          onClick={() => {
            setCompose('sale');
            setSettle(null);
          }}
        >
          New sales invoice
        </button>
        <button
          className="button secondary"
          onClick={() => {
            setCompose('purchase');
            setSettle(null);
          }}
        >
          New purchase invoice
        </button>
        <button className="button secondary" onClick={() => setShowParties(!showParties)}>
          Customers & suppliers
        </button>
        <Field name="Balances as of">
          <input
            type="date"
            value={props.asOf}
            onChange={(e) => {
              props.setOffset(0);
              props.setAsOf(e.target.value);
            }}
          />
        </Field>
      </div>
      {showParties && <Parties {...props} />}
      {compose && (
        <DocumentComposer
          key={`${compose}-${settle?.id || 0}`}
          {...props}
          kind={compose}
          invoice={settle}
          close={() => {
            setCompose(null);
            setSettle(null);
          }}
        />
      )}
      <section className="panel">
        <div className="panel-toolbar">
          <div className="tabs">
            {['all', 'sale', 'purchase', 'receipt', 'payment'].map((k) => (
              <button
                key={k}
                className={filter === k ? 'selected' : ''}
                onClick={() => setFilter(k)}
              >
                {docNames[k] || 'All documents'}
              </button>
            ))}
          </div>
          <span>{data.document_count} documents</span>
        </div>
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th>Document</th>
                <th>Party</th>
                <th>Due</th>
                <th>Status</th>
                <th className="numeric">Amount</th>
                <th className="numeric">Outstanding</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              {records.map((d) => (
                <tr key={d.id}>
                  <td>
                    <button className="table-link" onClick={() => setDetail(d)}>
                      {d.number}
                    </button>
                    <small className="cell-note">
                      {docNames[d.input.kind]} · {displayDate(d.input.date)}
                    </small>
                  </td>
                  <td>{data.parties.find((p) => p.id === d.input.party_id)?.input.name || '—'}</td>
                  <td>{d.input.due_date ? displayDate(d.input.due_date) : '—'}</td>
                  <td>
                    {d.reversed
                      ? 'Reversed'
                      : BigInt(d.outstanding_minor) > 0n
                        ? d.input.due_date! < (props.asOf || today())
                          ? 'Overdue'
                          : 'Open'
                        : 'Settled'}
                  </td>
                  <td className="numeric">{money(d.total_minor, currency)}</td>
                  <td className="numeric">{money(d.outstanding_minor, currency)}</td>
                  <td>
                    <div className="business-actions">
                      <button className="text-button" onClick={() => open(d.journal_id)}>
                        Journal
                      </button>
                      {!d.reversed && BigInt(d.outstanding_minor) > 0n && (
                        <button
                          className="text-button"
                          onClick={() => {
                            setSettle(d);
                            setCompose(d.input.kind === 'sale' ? 'receipt' : 'payment');
                          }}
                        >
                          Record {d.input.kind === 'sale' ? 'receipt' : 'payment'}
                        </button>
                      )}
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {!records.length && (
            <p className="panel-footnote">
              No documents on this page. Add a customer or supplier to get started.
            </p>
          )}
        </div>
        <Pager offset={props.offset} count={data.document_count} change={props.setOffset} />
      </section>
      {detail && (
        <section className="panel business-card">
          <div className="panel-title">
            <h2>{detail.number}</h2>
            <button className="button secondary" onClick={() => setDetail(null)}>
              Close detail
            </button>
          </div>
          <p>
            {state.company!.name} · {docNames[detail.input.kind]} · {displayDate(detail.input.date)}
          </p>
          <p>
            {data.parties.find((p) => p.id === detail.input.party_id)?.input.name} ·{' '}
            {data.parties.find((p) => p.id === detail.input.party_id)?.input.contact}
          </p>
          <DocumentLines lines={detail.input.lines} currency={currency} />
          {detail.gst && <GstAmounts calc={detail.gst} currency={currency} />}
          <p>{detail.input.note}</p>
          <strong>Total: {money(detail.total_minor, currency)}</strong>
          <div className="business-actions">
            <button
              className="button secondary"
              onClick={() => exportDocument(detail, data, state)}
            >
              Export document (HTML)
            </button>
            <button
              className="button secondary"
              onClick={() => exportDocument(detail, data, state, true)}
            >
              Print / save PDF
            </button>
          </div>
        </section>
      )}
    </>
  );
}
function Pager({
  offset,
  count,
  change,
}: {
  offset: number;
  count: number;
  change: (n: number) => void;
}) {
  return (
    <div className="panel-toolbar">
      <button
        className="button secondary"
        disabled={!offset}
        onClick={() => change(Math.max(0, offset - 50))}
      >
        Previous
      </button>
      <span>
        Page {offset / 50 + 1} · {count} records
      </span>
      <button
        className="button secondary"
        disabled={offset + 50 >= count}
        onClick={() => change(offset + 50)}
      >
        Next
      </button>
    </div>
  );
}
function Parties({ state, data, busy, mutate }: Props & { data: Business }) {
  const [kind, setKind] = useState('customer'),
    [name, setName] = useState(''),
    [contact, setContact] = useState(''),
    [account, setAccount] = useState(0);
  return (
    <section className="panel business-card">
      <h3>Customers & suppliers</h3>
      <form
        onSubmit={async (e) => {
          e.preventDefault();
          if (
            await mutate(
              'create_party',
              { name, kind, contact, account_id: account },
              'Party added.',
            )
          ) {
            setName('');
            setContact('');
          }
        }}
      >
        <div className="business-grid">
          <Field name="Party type">
            <select
              value={kind}
              onChange={(e) => {
                setKind(e.target.value);
                setAccount(0);
              }}
            >
              <option value="customer">Customer</option>
              <option value="supplier">Supplier</option>
            </select>
          </Field>
          <Field name="Name">
            <input
              required
              maxLength={120}
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </Field>
          <Field name="Contact / address">
            <input maxLength={500} value={contact} onChange={(e) => setContact(e.target.value)} />
          </Field>
          <AccountSelect
            accounts={state.accounts}
            kinds={[kind === 'customer' ? 'asset' : 'liability']}
            value={account}
            change={setAccount}
            label={kind === 'customer' ? 'Receivable account' : 'Payable account'}
          />
        </div>
        <button className="button primary" disabled={busy}>
          Add party
        </button>
      </form>
      <p className="form-hint">
        A shared control account is supported. Invoice balances track each party separately.
      </p>
      <div className="table-scroll">
        <table>
          <thead>
            <tr>
              <th>Name</th>
              <th>Type</th>
              <th>Contact</th>
              <th>Control account</th>
            </tr>
          </thead>
          <tbody>
            {data.parties.map((p) => (
              <tr key={p.id}>
                <td>{p.input.name}</td>
                <td>{p.input.kind}</td>
                <td>{p.input.contact}</td>
                <td>{state.accounts.find((a) => a.id === p.input.account_id)?.name}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}
function DocumentLines({ lines, currency }: { lines: DocLine[]; currency: string }) {
  return (
    <div className="table-scroll">
      <table>
        <thead>
          <tr>
            <th>Description</th>
            <th>Quantity</th>
            <th className="numeric">Unit price</th>
            <th className="numeric">Line total</th>
          </tr>
        </thead>
        <tbody>
          {lines.map((l, i) => (
            <tr key={i}>
              <td>{l.description}</td>
              <td>{l.quantity}</td>
              <td className="numeric">{money(parseMinor(l.unit_price) || 0n, currency)}</td>
              <td className="numeric">{money(lineTotal(l), currency)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
function DocumentComposer({
  kind,
  invoice,
  close,
  state,
  data,
  busy,
  mutate,
}: Props & { data: Business; kind: string; invoice: Doc | null; close: () => void }) {
  const settlement = kind === 'receipt' || kind === 'payment',
    stock = kind === 'stock';
  const [date, setDate] = useState(today()),
    [due, setDue] = useState(today()),
    [party, setParty] = useState(invoice?.input.party_id || 0),
    [bank, setBank] = useState(0),
    [amount, setAmount] = useState(invoice ? decimal(BigInt(invoice.outstanding_minor)) : ''),
    [note, setNote] = useState(''),
    [direction, setDirection] = useState('add');
  const [lines, setLines] = useState<ComposerLine[]>([
      { description: '', quantity: '1', unit_price: '', account_id: 0, item_id: null },
    ]),
    [review, setReview] = useState(false);
  const [requestId, setRequestId] = useState<string>(() => crypto.randomUUID());
  const patch = (i: number, p: Partial<ComposerLine>) =>
    setLines((v) => v.map((l, n) => (i === n ? { ...l, ...p } : l)));
  const [gstEnabled, setGstEnabled] = useState(false),
    [supplyState, setSupplyState] = useState(data.gst?.state_code || ''),
    [supplyName, setSupplyName] = useState(data.gst?.state_name || ''),
    [gstin, setGstin] = useState(''),
    [taxAddress, setTaxAddress] = useState(''),
    [delivery, setDelivery] = useState(''),
    [supplierRef, setSupplierRef] = useState(''),
    [creditReviewed, setCreditReviewed] = useState(false);
  const gstInput: GstInput = {
    supply_state: supplyState,
    supply_state_name: supplyName,
    counterparty_gstin: gstin || null,
    counterparty_address: taxAddress,
    delivery_address: delivery,
    supplier_reference: kind === 'purchase' ? supplierRef : null,
    credit_reviewed: kind === 'purchase' ? creditReviewed : false,
    rates_bps: lines.map((l) => Number(parseMinor(l.gst_rate || '') || 0n)),
    hsn_sac: lines.map((l) => l.hsn_sac || ''),
  };
  const gstPreview = () => {
    let c = 0n,
      i = 0n;
    const local = (kind === 'sale' ? data.gst?.state_code : gstin.slice(0, 2)) === supplyState;
    for (const l of lines) {
      const base = lineTotal(l),
        rate = parseMinor(l.gst_rate || '') || 0n;
      if (local) c += (base * rate + 10000n) / 20000n;
      else i += (base * rate + 5000n) / 10000n;
    }
    return {
      cgst_minor: c.toString(),
      sgst_minor: c.toString(),
      igst_minor: i.toString(),
      taxable_minor: lines.reduce((v, l) => v + lineTotal(l), 0n).toString(),
      lines: [],
    };
  };
  const calculatedGst = gstEnabled ? gstPreview() : null;
  const payload: DocInput = {
    ...(gstEnabled ? { gst: gstInput } : {}),
    kind,
    date,
    due_date: settlement || stock ? null : due,
    party_id: stock ? null : party,
    invoice_id: invoice?.id || null,
    money_account_id: settlement ? bank : null,
    amount: settlement ? amount : null,
    note,
    lines: settlement ? [] : lines.map(({ gst_rate, hsn_sac, ...line }) => line),
    request_id: requestId,
  };
  const total = settlement
    ? parseMinor(amount) || 0n
    : lines.reduce((v, l) => v + lineTotal(l), 0n) +
      (calculatedGst
        ? BigInt(calculatedGst.cgst_minor) +
          BigInt(calculatedGst.sgst_minor) +
          BigInt(calculatedGst.igst_minor)
        : 0n);
  const currency = state.company!.currency;
  const draftKey = `open-ledger-draft:${state.company!.created_at}:${state.company!.name}:${kind}`;
  const saveDraft = () => {
    localStorage.setItem(draftKey, JSON.stringify(payload));
  };
  const loadDraft = () => {
    try {
      const raw = localStorage.getItem(draftKey);
      if (!raw) return;
      const d = JSON.parse(raw) as DocInput;
      if (
        d.kind !== kind ||
        typeof d.date !== 'string' ||
        typeof d.note !== 'string' ||
        typeof d.request_id !== 'string' ||
        !Array.isArray(d.lines) ||
        !d.lines.every(
          (l) =>
            typeof l.description === 'string' &&
            typeof l.quantity === 'string' &&
            typeof l.unit_price === 'string' &&
            typeof l.account_id === 'number',
        )
      )
        return;
      setDate(d.date);
      setDue(d.due_date || today());
      setParty(d.party_id || 0);
      setNote(d.note);
      setLines(
        d.lines.map((l, i) => ({
          ...l,
          ...(d.gst
            ? { gst_rate: decimal(BigInt(d.gst.rates_bps[i])), hsn_sac: d.gst.hsn_sac[i] }
            : {}),
        })),
      );
      setGstEnabled(Boolean(d.gst));
      if (d.gst) {
        setSupplyState(d.gst.supply_state);
        setSupplyName(d.gst.supply_state_name);
        setGstin(d.gst.counterparty_gstin || '');
        setTaxAddress(d.gst.counterparty_address);
        setDelivery(d.gst.delivery_address);
        setSupplierRef(d.gst.supplier_reference || '');
        setCreditReviewed(d.gst.credit_reviewed);
      }
      setRequestId(d.request_id);
    } catch {
      /* A damaged local draft does not affect books. */
    }
  };
  return (
    <section className="panel business-card">
      <div className="panel-title">
        <h3>
          {docNames[kind]}
          {invoice ? ` for ${invoice.number}` : ''}
        </h3>
        <button className="button secondary" disabled={busy} onClick={close}>
          Cancel
        </button>
      </div>
      {review ? (
        <>
          <p>Review before posting. Posted documents are immutable.</p>
          <p>
            {displayDate(date)} · {data.parties.find((p) => p.id === party)?.input.name}{' '}
            {due && !settlement && !stock ? `· Due ${displayDate(due)}` : ''}
          </p>
          {!settlement && <DocumentLines lines={lines} currency={currency} />}
          {calculatedGst && <GstAmounts calc={calculatedGst} currency={currency} />}
          <p>{note}</p>
          <strong>
            {stock && direction === 'remove'
              ? 'Issue valued at current weighted average cost'
              : `Total: ${money(total, currency)}`}
          </strong>
          <div className="business-actions">
            <button
              className="button secondary"
              disabled={busy}
              onClick={() => {
                setReview(false);
                setRequestId(crypto.randomUUID());
              }}
            >
              Edit
            </button>
            <button
              className="button primary"
              disabled={busy}
              onClick={async () => {
                if (await mutate('post_document', payload, 'Document posted.')) {
                  localStorage.removeItem(draftKey);
                  close();
                }
              }}
            >
              Post {docNames[kind].toLowerCase()}
            </button>
          </div>
        </>
      ) : (
        <form
          onSubmit={(e) => {
            e.preventDefault();
            setReview(true);
          }}
        >
          <div className="business-grid">
            <Field name="Date">
              <input type="date" required value={date} onChange={(e) => setDate(e.target.value)} />
            </Field>
            {!settlement && !stock && (
              <Field name="Due date">
                <input
                  type="date"
                  required
                  min={date}
                  value={due}
                  onChange={(e) => setDue(e.target.value)}
                />
              </Field>
            )}
            {!stock && (
              <Field name={kind === 'sale' || kind === 'receipt' ? 'Customer' : 'Supplier'}>
                <select
                  required
                  disabled={settlement}
                  value={party || ''}
                  onChange={(e) => setParty(Number(e.target.value))}
                >
                  <option value="">Select party</option>
                  {data.parties
                    .filter(
                      (p) =>
                        p.input.kind ===
                        (kind === 'sale' || kind === 'receipt' ? 'customer' : 'supplier'),
                    )
                    .map((p) => (
                      <option key={p.id} value={p.id}>
                        {p.input.name}
                      </option>
                    ))}
                </select>
              </Field>
            )}
            {stock && (
              <Field name="Movement">
                <select
                  value={direction}
                  onChange={(e) => {
                    setDirection(e.target.value);
                    setLines([
                      {
                        description: '',
                        quantity: '1',
                        unit_price: '1',
                        account_id: 0,
                        item_id: null,
                      },
                    ]);
                  }}
                >
                  <option value="add">Stock received / opening stock</option>
                  <option value="remove">Stock issued / adjustment</option>
                </select>
              </Field>
            )}
            {settlement && (
              <>
                <AccountSelect
                  accounts={state.accounts}
                  kinds={['asset']}
                  value={bank}
                  change={setBank}
                  label="Cash / bank account"
                />
                <Field name="Amount">
                  <input
                    required
                    inputMode="decimal"
                    value={amount}
                    onChange={(e) => setAmount(e.target.value)}
                  />
                </Field>
              </>
            )}
            <Field name="Note">
              <input maxLength={500} value={note} onChange={(e) => setNote(e.target.value)} />
            </Field>
          </div>
          {!settlement && !stock && data.gst && (
            <>
              <label className="checkbox-label">
                <input
                  type="checkbox"
                  checked={gstEnabled}
                  onChange={(e) => setGstEnabled(e.target.checked)}
                />
                Calculate India regular GST
              </label>
              {gstEnabled && (
                <fieldset className="document-line">
                  <legend>Domestic GST particulars</legend>
                  <div className="business-grid">
                    <Field name="Place of supply state code">
                      <input
                        required
                        maxLength={2}
                        value={supplyState}
                        onChange={(e) => setSupplyState(e.target.value)}
                      />
                    </Field>
                    <Field name="Place of supply state name">
                      <input
                        required
                        maxLength={80}
                        value={supplyName}
                        onChange={(e) => setSupplyName(e.target.value)}
                      />
                    </Field>
                    <Field
                      name={kind === 'sale' ? 'Recipient GSTIN (if registered)' : 'Supplier GSTIN'}
                    >
                      <input
                        required={kind === 'purchase'}
                        maxLength={15}
                        value={gstin}
                        onChange={(e) => setGstin(e.target.value.toUpperCase())}
                      />
                    </Field>
                    <Field name="Counterparty postal address">
                      <input
                        required
                        maxLength={500}
                        value={taxAddress}
                        onChange={(e) => setTaxAddress(e.target.value)}
                      />
                    </Field>
                    <Field name="Delivery address">
                      <input
                        required
                        maxLength={500}
                        value={delivery}
                        onChange={(e) => setDelivery(e.target.value)}
                      />
                    </Field>
                    {kind === 'purchase' && (
                      <Field name="Supplier tax invoice reference">
                        <input
                          required
                          maxLength={16}
                          value={supplierRef}
                          onChange={(e) => setSupplierRef(e.target.value)}
                        />
                      </Field>
                    )}
                  </div>
                  {kind === 'purchase' && (
                    <label className="checkbox-label">
                      <input
                        required
                        type="checkbox"
                        checked={creditReviewed}
                        onChange={(e) => setCreditReviewed(e.target.checked)}
                      />
                      I independently reviewed input tax credit eligibility.
                    </label>
                  )}
                  <p className="form-hint">
                    Domestic forward-charge supplies only. Exports, SEZ, reverse charge,
                    composition, cess, e-invoicing and statutory return filing require separate
                    workflows.
                  </p>
                </fieldset>
              )}
            </>
          )}
          {!settlement && (
            <>
              {lines.map((l, i) => (
                <fieldset key={i} className="document-line">
                  <legend>Line {i + 1}</legend>
                  <div className="business-grid">
                    <Field name="Inventory item">
                      <select
                        value={l.item_id || ''}
                        required={stock}
                        onChange={(e) => {
                          const item = data.items.find((a) => a.id === Number(e.target.value));
                          patch(i, {
                            item_id: item?.id || null,
                            description: item?.input.name || l.description,
                            account_id:
                              item && (kind === 'purchase' || (stock && direction === 'remove'))
                                ? item.input.asset_account_id
                                : 0,
                          });
                        }}
                      >
                        <option value="">
                          {stock ? 'Select item' : 'Service / non-stock line'}
                        </option>
                        {data.items.map((item) => (
                          <option key={item.id} value={item.id}>
                            {item.input.code} · {item.input.name} ({item.input.unit})
                          </option>
                        ))}
                      </select>
                    </Field>
                    <Field name="Description">
                      <input
                        required
                        maxLength={200}
                        value={l.description}
                        onChange={(e) => patch(i, { description: e.target.value })}
                      />
                    </Field>
                    {gstEnabled && (
                      <>
                        <Field name="Reviewed GST rate (%)">
                          <input
                            required
                            inputMode="decimal"
                            value={l.gst_rate || ''}
                            onChange={(e) => patch(i, { gst_rate: e.target.value })}
                          />
                        </Field>
                        <Field name="HSN / SAC">
                          <input
                            required
                            maxLength={8}
                            value={l.hsn_sac || ''}
                            onChange={(e) => patch(i, { hsn_sac: e.target.value })}
                          />
                        </Field>
                      </>
                    )}
                    <Field name="Quantity (up to 3 decimals)">
                      <input
                        required
                        inputMode="decimal"
                        value={l.quantity}
                        onChange={(e) => patch(i, { quantity: e.target.value })}
                      />
                    </Field>
                    {!(stock && direction === 'remove') && (
                      <Field name="Unit price">
                        <input
                          required
                          inputMode="decimal"
                          value={l.unit_price}
                          onChange={(e) => patch(i, { unit_price: e.target.value })}
                        />
                      </Field>
                    )}
                    {!(stock && direction === 'remove') && (
                      <AccountSelect
                        accounts={state.accounts}
                        kinds={
                          kind === 'sale'
                            ? ['income']
                            : stock
                              ? ['equity', 'income', 'expense']
                              : ['expense', 'asset']
                        }
                        value={l.account_id}
                        change={(id) => patch(i, { account_id: id })}
                        label={stock ? 'Offset account' : 'Posting account'}
                      />
                    )}
                  </div>
                  <button
                    type="button"
                    className="text-button"
                    disabled={lines.length === 1}
                    onClick={() => setLines((v) => v.filter((_, n) => n !== i))}
                  >
                    Remove line
                  </button>
                </fieldset>
              ))}
              <button
                type="button"
                className="button secondary"
                disabled={lines.length >= 100}
                onClick={() =>
                  setLines((v) => [
                    ...v,
                    {
                      description: '',
                      quantity: '1',
                      unit_price: stock && direction === 'remove' ? '1' : '',
                      account_id: 0,
                      item_id: null,
                    },
                  ])
                }
              >
                Add line
              </button>
            </>
          )}
          <div className="business-actions">
            <button className="button primary" disabled={busy}>
              Review document
            </button>
            {!settlement && !stock && (
              <>
                <button type="button" className="button secondary" onClick={saveDraft}>
                  Save draft on this device
                </button>
                <button type="button" className="button secondary" onClick={loadDraft}>
                  Load saved draft
                </button>
              </>
            )}
          </div>
          <p className="form-hint">
            Amounts use two decimal places. Stock uses perpetual weighted average cost. Taxes are
            not calculated in this general accounting workflow.
          </p>
        </form>
      )}
    </section>
  );
}
function Inventory(props: Props & { data: Business; asOf: string; setAsOf: (s: string) => void }) {
  const { state, data, busy, mutate } = props;
  const [code, setCode] = useState(''),
    [name, setName] = useState(''),
    [unit, setUnit] = useState('each'),
    [asset, setAsset] = useState(0),
    [expense, setExpense] = useState(0),
    [compose, setCompose] = useState(false);
  return (
    <>
      <section className="panel filter-panel">
        <button className="button primary" onClick={() => setCompose(!compose)}>
          Stock adjustment
        </button>
        <Field name="Stock as of">
          <input type="date" value={props.asOf} onChange={(e) => props.setAsOf(e.target.value)} />
        </Field>
        <button
          className="button secondary"
          onClick={() =>
            download(
              `stock-${props.asOf || today()}.csv`,
              csv([
                ['Code', 'Item', 'Unit', 'Quantity', 'Value minor'],
                ...data.stock.map((s) => [
                  s.item.input.code,
                  s.item.input.name,
                  s.item.input.unit,
                  quantityText(s.quantity_milli),
                  s.value_minor,
                ]),
              ]),
              'text/csv',
            )
          }
        >
          Export stock
        </button>
      </section>
      {compose && (
        <DocumentComposer {...props} kind="stock" invoice={null} close={() => setCompose(false)} />
      )}
      <section className="panel">
        <div className="panel-title">
          <h3>Stock on hand · weighted average valuation</h3>
        </div>
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th>Code</th>
                <th>Item</th>
                <th>Unit</th>
                <th className="numeric">Quantity</th>
                <th className="numeric">Carrying value</th>
              </tr>
            </thead>
            <tbody>
              {data.stock.map((s) => (
                <tr key={s.item.id}>
                  <td>{s.item.input.code}</td>
                  <td>{s.item.input.name}</td>
                  <td>{s.item.input.unit}</td>
                  <td className="numeric">{quantityText(s.quantity_milli)}</td>
                  <td className="numeric">{money(s.value_minor, state.company!.currency)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>
      <section className="panel business-card">
        <h3>Add inventory item</h3>
        <p className="form-hint">
          Create a dedicated inventory asset account and a cost-of-sales expense account in the
          chart of accounts. Existing stock accounts may be shared between items.
        </p>
        <form
          onSubmit={async (e) => {
            e.preventDefault();
            if (
              await mutate(
                'create_item',
                { code, name, unit, asset_account_id: asset, expense_account_id: expense },
                'Stock item added.',
              )
            ) {
              setCode('');
              setName('');
            }
          }}
        >
          <div className="business-grid">
            <Field name="Item code">
              <input
                required
                maxLength={20}
                value={code}
                onChange={(e) => setCode(e.target.value)}
              />
            </Field>
            <Field name="Item name">
              <input
                required
                maxLength={120}
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
            </Field>
            <Field name="Unit">
              <input
                required
                maxLength={20}
                value={unit}
                onChange={(e) => setUnit(e.target.value)}
              />
            </Field>
            <AccountSelect
              accounts={state.accounts}
              kinds={['asset']}
              value={asset}
              change={setAsset}
              label="Inventory asset account"
            />
            <AccountSelect
              accounts={state.accounts}
              kinds={['expense']}
              value={expense}
              change={setExpense}
              label="Cost / adjustment expense account"
            />
          </div>
          <button className="button primary" disabled={busy}>
            Add item
          </button>
        </form>
      </section>
      <StockHistory {...props} />
    </>
  );
}
type StockHistoryData = {
  count: number;
  rows: {
    id: number;
    name: string;
    unit: string;
    number: string;
    date: string;
    journal_id: number;
    quantity_milli: string;
    value_minor: string;
  }[];
};
function StockHistory({ state, onError, openJournal }: Props) {
  const [offset, setOffset] = useState(0),
    [data, setData] = useState<StockHistoryData | null>(null);
  useEffect(() => {
    let live = true;
    void request<StockHistoryData>('stock_history', { offset, limit: 50 })
      .then((v) => {
        if (live) setData(v);
      })
      .catch((e) => onError(err(e)));
    return () => {
      live = false;
    };
  }, [state, offset]);
  if (!data) return null;
  return (
    <section className="panel">
      <div className="panel-title">
        <h3>Stock movement audit</h3>
      </div>
      <div className="table-scroll">
        <table>
          <thead>
            <tr>
              <th>Date</th>
              <th>Document</th>
              <th>Item</th>
              <th className="numeric">Quantity</th>
              <th className="numeric">Value</th>
            </tr>
          </thead>
          <tbody>
            {data.rows.map((r) => (
              <tr key={r.id}>
                <td>{displayDate(r.date)}</td>
                <td>
                  <button
                    className="table-link"
                    onClick={() =>
                      void request<Journal>('journal', { id: r.journal_id })
                        .then(openJournal)
                        .catch((e) => onError(err(e)))
                    }
                  >
                    {r.number}
                  </button>
                </td>
                <td>
                  {r.name} ({r.unit})
                </td>
                <td className="numeric">{quantityText(r.quantity_milli)}</td>
                <td className="numeric">{money(r.value_minor, state.company!.currency)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <Pager offset={offset} count={data.count} change={setOffset} />
    </section>
  );
}
type StatementsData = {
  profit_loss: { code: string; name: string; kind: string; amount_minor: string }[];
  balance_sheet: { code: string; name: string; kind: string; amount_minor: string }[];
  income_minor: string;
  expense_minor: string;
  profit_minor: string;
  assets_minor: string;
  liabilities_minor: string;
  equity_minor: string;
  current_earnings_minor: string;
  balanced: boolean;
};
function Statements({ state, onError, gstConfigured }: Props & { gstConfigured: boolean }) {
  const [from, setFrom] = useState(state.company!.fiscal_year_start),
    [asOf, setAsOf] = useState(today()),
    [report, setReport] = useState<StatementsData | null>(null);
  useEffect(() => {
    let live = true;
    void request<StatementsData>('financial_statements', {
      from: from || null,
      as_of: asOf || null,
    })
      .then((r) => {
        if (live) setReport(r);
      })
      .catch((e) => {
        if (live) onError(err(e));
      });
    return () => {
      live = false;
    };
  }, [from, asOf, state]);
  const currency = state.company!.currency;
  if (!report) return <p>Loading statements…</p>;
  const exportReport = () =>
    download(
      'financial-statements.csv',
      csv([
        ['Report', 'Code', 'Account', 'Type', 'Amount minor'],
        ...report.profit_loss.map((r) => [
          'Profit and loss',
          r.code,
          r.name,
          r.kind,
          r.amount_minor,
        ]),
        ...report.balance_sheet.map((r) => [
          'Balance sheet',
          r.code,
          r.name,
          r.kind,
          r.amount_minor,
        ]),
        ['Balance sheet', '', 'Current earnings', 'equity', report.current_earnings_minor],
        ['Profit and loss', '', 'Net profit', 'total', report.profit_minor],
      ]),
      'text/csv',
    );
  return (
    <>
      {gstConfigured && <GstReport state={state} onError={onError} />}
      <section className="panel filter-panel">
        <Field name="Profit and loss from">
          <input type="date" value={from} onChange={(e) => setFrom(e.target.value)} />
        </Field>
        <Field name="As of">
          <input type="date" value={asOf} onChange={(e) => setAsOf(e.target.value)} />
        </Field>
        <button className="button secondary" onClick={exportReport}>
          Export statements
        </button>
      </section>
      <div className="metrics">
        {[
          ['Income', report.income_minor],
          ['Expenses', report.expense_minor],
          ['Net profit', report.profit_minor],
          ['Total assets', report.assets_minor],
        ].map(([name, value]) => (
          <div className="metric" key={name}>
            <div>{name}</div>
            <strong>{money(value, currency)}</strong>
          </div>
        ))}
      </div>
      {[
        ['Profit and loss', report.profit_loss],
        ['Balance sheet', report.balance_sheet],
      ].map(([name, rows]) => (
        <section className="panel" key={name as string}>
          <div className="panel-title">
            <h3>{name as string}</h3>
          </div>
          <div className="table-scroll">
            <table>
              <thead>
                <tr>
                  <th>Code</th>
                  <th>Account</th>
                  <th>Type</th>
                  <th className="numeric">Amount</th>
                </tr>
              </thead>
              <tbody>
                {(rows as StatementsData['profit_loss']).map((r) => (
                  <tr key={r.code}>
                    <td>{r.code}</td>
                    <td>{r.name}</td>
                    <td>{r.kind}</td>
                    <td className="numeric">{money(r.amount_minor, currency)}</td>
                  </tr>
                ))}
                {name === 'Balance sheet' && (
                  <tr>
                    <td>—</td>
                    <td>Current earnings (unclosed)</td>
                    <td>equity</td>
                    <td className="numeric">{money(report.current_earnings_minor, currency)}</td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
          {name === 'Balance sheet' && (
            <p className="panel-footnote">
              Assets {money(report.assets_minor, currency)} = liabilities{' '}
              {money(report.liabilities_minor, currency)} + equity{' '}
              {money(report.equity_minor, currency)} + current earnings{' '}
              {money(report.current_earnings_minor, currency)} ·{' '}
              {report.balanced ? 'Balanced' : 'Out of balance'}
            </p>
          )}
        </section>
      ))}
      <p className="form-hint">
        Profit and loss excludes the transfer entries generated when periods close. The balance
        sheet includes retained earnings and any unclosed earnings.
      </p>
    </>
  );
}
type BankLine = {
  id: number;
  journal_id: number;
  date: string;
  reference: string;
  description: string;
  net_minor: string;
  reconciliation_id: number | null;
};
function Bank({ state, data, busy, mutate, onError }: Props & { data: Business }) {
  const [account, setAccount] = useState(state.accounts.find((a) => a.code === '1100')?.id || 0),
    [start, setStart] = useState(state.company!.fiscal_year_start),
    [end, setEnd] = useState(today()),
    [opening, setOpening] = useState('0.00'),
    [closing, setClosing] = useState(''),
    [reference, setReference] = useState(''),
    [lines, setLines] = useState<BankLine[]>([]),
    [selected, setSelected] = useState<number[]>([]),
    [requestId, setRequestId] = useState<string>(() => crypto.randomUUID());
  useEffect(() => {
    let live = true;
    setSelected([]);
    setRequestId(crypto.randomUUID());
    void request<BankLine[]>('bank_lines', { account_id: account, as_of: end || null })
      .then((v) => {
        if (live) setLines(v);
      })
      .catch((e) => {
        if (live) onError(err(e));
      });
    return () => {
      live = false;
    };
  }, [account, end, state]);
  const selectedNet = lines
      .filter((l) => selected.includes(l.id))
      .reduce((n, l) => n + BigInt(l.net_minor), 0n),
    book = lines.reduce((n, l) => n + BigInt(l.net_minor), 0n),
    outstanding = lines
      .filter((l) => l.reconciliation_id === null && !selected.includes(l.id))
      .reduce((n, l) => n + BigInt(l.net_minor), 0n);
  const signed = (s: string) =>
    s.startsWith('-') ? -(parseMinor(s.slice(1)) || 0n) : parseMinor(s) || 0n;
  return (
    <>
      <section className="panel business-card">
        <h3>Reconcile bank statement</h3>
        <form
          onSubmit={async (e) => {
            e.preventDefault();
            if (
              await mutate(
                'reconcile',
                {
                  account_id: account,
                  start,
                  end,
                  opening,
                  closing,
                  reference,
                  line_ids: selected,
                  request_id: requestId,
                },
                'Statement reconciled.',
              )
            ) {
              setSelected([]);
              setRequestId(crypto.randomUUID());
            }
          }}
        >
          <div className="business-grid">
            <AccountSelect
              accounts={state.accounts}
              kinds={['asset']}
              value={account}
              change={setAccount}
              label="Bank account"
            />
            <Field name="Statement from">
              <input
                required
                type="date"
                value={start}
                onChange={(e) => setStart(e.target.value)}
              />
            </Field>
            <Field name="Statement through">
              <input
                required
                type="date"
                min={start}
                value={end}
                onChange={(e) => setEnd(e.target.value)}
              />
            </Field>
            <Field name="Statement opening">
              <input required value={opening} onChange={(e) => setOpening(e.target.value)} />
            </Field>
            <Field name="Statement closing">
              <input required value={closing} onChange={(e) => setClosing(e.target.value)} />
            </Field>
            <Field name="Statement reference">
              <input
                required
                maxLength={120}
                value={reference}
                onChange={(e) => setReference(e.target.value)}
              />
            </Field>
          </div>
          <p className="form-hint">
            Select transactions appearing on the statement. Earlier outstanding transactions can
            clear here. A negative bank balance uses a minus sign.
          </p>
          <div className="table-scroll">
            <table>
              <thead>
                <tr>
                  <th>Cleared</th>
                  <th>Date</th>
                  <th>Reference</th>
                  <th>Description</th>
                  <th className="numeric">Bank movement</th>
                </tr>
              </thead>
              <tbody>
                {lines.map((l) => (
                  <tr key={l.id}>
                    <td>
                      <input
                        type="checkbox"
                        aria-label={`Clear ${l.reference} line ${l.id}`}
                        disabled={l.reconciliation_id !== null}
                        checked={l.reconciliation_id !== null || selected.includes(l.id)}
                        onChange={(e) =>
                          setSelected((v) =>
                            e.target.checked ? [...v, l.id] : v.filter((n) => n !== l.id),
                          )
                        }
                      />
                    </td>
                    <td>{displayDate(l.date)}</td>
                    <td>{l.reference}</td>
                    <td>{l.description}</td>
                    <td className="numeric">{money(l.net_minor, state.company!.currency)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="business-actions">
            <strong>
              Statement difference:{' '}
              {money(signed(opening) + selectedNet - signed(closing), state.company!.currency)}
            </strong>
            <button className="button primary" disabled={busy}>
              Save reconciliation
            </button>
          </div>
          <p className="form-hint">
            Book balance: {money(book, state.company!.currency)} · Outstanding book movements after
            selection: {money(outstanding, state.company!.currency)}
          </p>
        </form>
      </section>
      <section className="panel">
        <div className="panel-title">
          <h3>Statement history</h3>
        </div>
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th>Reference</th>
                <th>Dates</th>
                <th>Opening</th>
                <th>Closing</th>
                <th>Cleared lines</th>
              </tr>
            </thead>
            <tbody>
              {data.reconciliations
                .filter((r) => r.input.account_id === account)
                .map((r) => (
                  <tr key={r.id}>
                    <td>{r.input.reference}</td>
                    <td>
                      {r.input.start} – {r.input.end}
                    </td>
                    <td>{r.input.opening}</td>
                    <td>{r.input.closing}</td>
                    <td>{r.input.line_ids.length}</td>
                  </tr>
                ))}
            </tbody>
          </table>
        </div>
      </section>
    </>
  );
}
function Settings(props: Props & { data: Business }) {
  const { state, data, busy, mutate, onError } = props;
  const last = data.periods.at(-1);
  const nextStart = last
    ? new Date(new Date(`${last.end}T12:00:00Z`).getTime() + 86400000).toISOString().slice(0, 10)
    : state.company!.fiscal_year_start;
  const [end, setEnd] = useState(''),
    [equity, setEquity] = useState(0),
    [ack, setAck] = useState(false),
    [bookName, setBookName] = useState(''),
    [books, setBooks] = useState<{ file: string; company: Snapshot['company'] }[]>([]),
    [active, setActive] = useState(''),
    [importText, setImportText] = useState(''),
    [preview, setPreview] = useState<{ count: number } | null>(null);
  useEffect(() => {
    void request<{ books: typeof books; active: string }>('books')
      .then((v) => {
        setBooks(v.books);
        setActive(v.active);
      })
      .catch((e) => onError(err(e)));
  }, [state]);
  return (
    <>
      {state.company!.currency === 'INR' && <GstSettings {...props} />}
      <TallyImporter {...props} />
      <section className="panel business-card">
        <h3>Company books</h3>
        <div className="business-grid">
          <Field name="Switch company">
            <select
              value={active}
              disabled={busy}
              onChange={(e) =>
                void mutate('switch_books', { file: e.target.value }, 'Company switched.')
              }
            >
              <option value="">Select books</option>
              {books.map((b) => (
                <option key={b.file} value={b.file}>
                  {b.company?.name || 'Empty books'}
                </option>
              ))}
            </select>
          </Field>
          <Field name="New company name">
            <input value={bookName} maxLength={120} onChange={(e) => setBookName(e.target.value)} />
          </Field>
        </div>
        <div className="business-actions">
          <button
            className="button primary"
            disabled={busy || !bookName.trim()}
            onClick={() =>
              void mutate(
                'new_books',
                { name: bookName, currency: state.company!.currency, fiscal_year_start: today() },
                'New company created.',
              )
            }
          >
            Create separate company
          </button>
          <button
            className="button secondary"
            disabled={busy}
            onClick={async () => {
              if (await mutate('new_books', {}, 'Separate example file created.'))
                await mutate('create_demo', {}, 'Example books opened.');
            }}
          >
            Open separate example books
          </button>
        </div>
        <p className="form-hint">
          Each company has its own database. Example books use a separate file. Use Backups to
          export the active company.
        </p>
      </section>
      <section className="panel business-card">
        <h3>Document numbering</h3>
        <p className="form-hint">Duplicate numbers are rejected before anything posts.</p>
        {data.numbering.map((n) => (
          <NumberSequence
            key={`${n.kind}-${n.prefix}-${n.next}`}
            number={n}
            busy={busy}
            mutate={mutate}
          />
        ))}
      </section>
      <section className="panel business-card">
        <h3>Close financial period</h3>
        <p className="form-hint">
          Closing transfers income and expense balances to the equity account you choose. Asset,
          liability, and equity balances carry forward automatically. Closed dates cannot accept new
          postings or reversals.
        </p>
        <form
          onSubmit={async (e) => {
            e.preventDefault();
            if (
              await mutate(
                'close_period',
                { start: nextStart, end, equity_account_id: equity },
                'Period closed.',
              )
            ) {
              setAck(false);
              setEnd('');
            }
          }}
        >
          <div className="business-grid">
            <Field name="Period begins">
              <input readOnly value={nextStart} />
            </Field>
            <Field name="Period ends">
              <input
                type="date"
                required
                min={nextStart}
                value={end}
                onChange={(e) => setEnd(e.target.value)}
              />
            </Field>
            <AccountSelect
              accounts={state.accounts}
              kinds={['equity']}
              value={equity}
              change={setEquity}
              label="Retained earnings / equity account"
            />
          </div>
          <label className="checkbox-label">
            <input
              required
              type="checkbox"
              checked={ack}
              onChange={(e) => setAck(e.target.checked)}
            />
            I reviewed the period's reports and saved a backup.
          </label>
          <button className="button primary" disabled={busy || !ack}>
            Close period
          </button>
        </form>
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th>From</th>
                <th>Through</th>
                <th>Closing journal</th>
              </tr>
            </thead>
            <tbody>
              {data.periods.map((p) => (
                <tr key={p.id}>
                  <td>{p.start}</td>
                  <td>{p.end}</td>
                  <td>{p.journal_id || 'No earnings to transfer'}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>
      <section className="panel business-card">
        <h3>Import journals</h3>
        <p className="form-hint">
          Use the documented Open Ledger JSON journal format. Preview validates the entire batch
          without changing your books. Native Tally files are not supported by this importer.
        </p>
        <Field name="Journal import JSON">
          <textarea
            rows={8}
            value={importText}
            onChange={(e) => {
              setImportText(e.target.value);
              setPreview(null);
            }}
          />
        </Field>
        <div className="business-actions">
          <button
            className="button secondary"
            disabled={busy}
            onClick={async () => {
              try {
                const v = JSON.parse(importText);
                setPreview(await request('import_journals', { ...v, preview: true }));
              } catch (e) {
                onError(err(e));
              }
            }}
          >
            Validate import
          </button>
          {preview && (
            <>
              <span>{preview.count} validated journals</span>
              <button
                className="button primary"
                disabled={busy}
                onClick={async () => {
                  try {
                    if (
                      await mutate(
                        'import_journals',
                        { ...JSON.parse(importText), preview: false },
                        'Journals imported.',
                      )
                    ) {
                      setPreview(null);
                      setImportText('');
                    }
                  } catch (e) {
                    onError(err(e));
                  }
                }}
              >
                Import validated batch
              </button>
            </>
          )}
        </div>
      </section>
      <section className="panel business-card">
        <h3>Display preferences</h3>
        <Field name="Number and date locale">
          <select
            defaultValue={localStorage.getItem('open-ledger-locale') || navigator.language}
            onChange={(e) => {
              localStorage.setItem('open-ledger-locale', e.target.value);
              window.location.reload();
            }}
          >
            {Array.from(
              new Set([navigator.language, 'en-IN', 'en-GB', 'en-US', 'de-DE', 'fr-FR']),
            ).map((l) => (
              <option key={l} value={l}>
                {l}
              </option>
            ))}
          </select>
        </Field>
      </section>
    </>
  );
}
function NumberSequence({
  number,
  busy,
  mutate,
}: {
  number: Numbering;
  busy: boolean;
  mutate: Mutation;
}) {
  const [prefix, setPrefix] = useState(number.prefix),
    [next, setNext] = useState(String(number.next));
  return (
    <form
      className="numbering-form"
      onSubmit={(e) => {
        e.preventDefault();
        void mutate(
          'set_numbering',
          { kind: number.kind, prefix, next: Number(next) },
          'Numbering saved.',
        );
      }}
    >
      <strong>{docNames[number.kind]}</strong>
      <Field name="Prefix">
        <input required maxLength={32} value={prefix} onChange={(e) => setPrefix(e.target.value)} />
      </Field>
      <Field name="Next number">
        <input
          type="number"
          min={1}
          max={Number.MAX_SAFE_INTEGER}
          required
          value={next}
          onChange={(e) => setNext(e.target.value)}
        />
      </Field>
      <button className="button secondary" disabled={busy}>
        Save
      </button>
    </form>
  );
}
function exportDocument(d: Doc, data: Business, state: Snapshot, print = false) {
  const escape = (s: string) =>
    s
      .replaceAll('&', '&amp;')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;')
      .replaceAll('"', '&quot;');
  const party = data.parties.find((p) => p.id === d.input.party_id);
  const currency = state.company!.currency;
  const g = d.input.gst;
  const gstHtml =
    d.gst && g && data.gst
      ? `<h3>India GST invoice particulars</h3><p>Supplier: ${escape(d.input.kind === 'sale' ? state.company!.name : party?.input.name || '')} · Registration: ${escape(d.input.kind === 'sale' ? data.gst.gstin : g.counterparty_gstin || '')} · ${escape(d.input.kind === 'sale' ? data.gst.address : g.counterparty_address)}</p><p>Recipient registration: ${escape(d.input.kind === 'sale' ? g.counterparty_gstin || 'Unregistered' : data.gst.gstin)} · ${escape(d.input.kind === 'sale' ? g.counterparty_address : data.gst.address)}</p><p>Place of supply: ${escape(g.supply_state_name)} (${escape(g.supply_state)}) · Delivery: ${escape(g.delivery_address)}</p>${g.supplier_reference ? `<p>Supplier invoice: ${escape(g.supplier_reference)}</p>` : ''}<table><thead><tr><th>HSN/SAC</th><th>Rate %</th><th>Taxable</th><th>CGST</th><th>SGST/UTGST</th><th>IGST</th></tr></thead><tbody>${d.gst.lines.map((l) => `<tr><td>${escape(l.hsn_sac)}</td><td>${escape(decimal(BigInt(l.rate_bps)))}</td><td>${escape(money(l.taxable_minor, currency))}</td><td>${escape(money(l.cgst_minor, currency))}</td><td>${escape(money(l.sgst_minor, currency))}</td><td>${escape(money(l.igst_minor, currency))}</td></tr>`).join('')}</tbody></table><p>Taxable: ${escape(money(d.gst.taxable_minor, currency))} · CGST: ${escape(money(d.gst.cgst_minor, currency))} · SGST/UTGST: ${escape(money(d.gst.sgst_minor, currency))} · IGST: ${escape(money(d.gst.igst_minor, currency))}</p><p>Reverse charge: No · Supplier authorization/signature: __________________</p>`
      : '';
  const html = `<!doctype html><html lang="en"><meta charset="utf-8"><title>${escape(d.number)}</title><style>body{font:16px system-ui;color:#20372e;max-width:850px;margin:48px auto;padding:24px}table{width:100%;border-collapse:collapse;margin:32px 0}td,th{text-align:left;border-bottom:1px solid #ddd;padding:12px}small{color:#667}</style><h1>${escape(state.company!.name)}</h1><h2>${escape(docNames[d.input.kind])} ${escape(d.number)}</h2><p>${escape(d.input.date)}${d.input.due_date ? ` · Due ${escape(d.input.due_date)}` : ''}${d.reversed ? ' · REVERSED' : ''}</p><h3>${escape(party?.input.name || '')}</h3><p>${escape(party?.input.contact || '')}</p><table><thead><tr><th>Description</th><th>Quantity</th><th>Unit price</th><th>Amount</th></tr></thead><tbody>${d.input.lines.map((l) => `<tr><td>${escape(l.description)}</td><td>${escape(l.quantity)} ${escape(data.items.find((i) => i.id === l.item_id)?.input.unit || '')}</td><td>${escape(money(parseMinor(l.unit_price) || 0n, currency))}</td><td>${escape(money(lineTotal(l), currency))}</td></tr>`).join('')}</tbody></table>${gstHtml}<h3>Total ${escape(money(d.total_minor, currency))}</h3><p>${escape(d.input.note)}</p><small>${d.gst ? 'Open Ledger · Domestic regular GST accounting · Review legal particulars and supplier authorization before issue' : 'Open Ledger · General accounting document · No GST calculation'}</small></html>`;
  if (print) {
    const frame = document.createElement('iframe');
    frame.className = 'print-frame';
    frame.setAttribute('title', 'Printable accounting document');
    frame.srcdoc = html;
    frame.onload = () => {
      frame.contentWindow?.focus();
      frame.contentWindow?.print();
      setTimeout(() => frame.remove(), 60000);
    };
    document.body.append(frame);
  } else download(`${d.number.replace(/[^a-z0-9_-]/gi, '_')}.html`, html, 'text/html');
}

function GstAmounts({ calc, currency }: { calc: GstCalculation; currency: string }) {
  return (
    <div className="ledger-summary">
      {[
        ['Taxable', calc.taxable_minor],
        ['CGST', calc.cgst_minor],
        ['SGST / UTGST', calc.sgst_minor],
        ['IGST', calc.igst_minor],
      ].map(([name, value]) => (
        <div key={name}>
          <span>{name}</span>
          <strong>{money(value, currency)}</strong>
        </div>
      ))}
    </div>
  );
}
function GstSettings({ state, data, busy, mutate }: Props & { data: Business }) {
  const [gstin, setGstin] = useState(''),
    [code, setCode] = useState(''),
    [name, setName] = useState(''),
    [address, setAddress] = useState('');
  const [output, setOutput] = useState<GstAccounts>({ cgst: 0, sgst: 0, igst: 0 }),
    [input, setInput] = useState<GstAccounts>({ cgst: 0, sgst: 0, igst: 0 });
  useEffect(() => {
    if (!data.gst) {
      setOutput({
        cgst: state.accounts.find((a) => a.code === '2101')?.id || 0,
        sgst: state.accounts.find((a) => a.code === '2102')?.id || 0,
        igst: state.accounts.find((a) => a.code === '2103')?.id || 0,
      });
      setInput({
        cgst: state.accounts.find((a) => a.code === '1401')?.id || 0,
        sgst: state.accounts.find((a) => a.code === '1402')?.id || 0,
        igst: state.accounts.find((a) => a.code === '1403')?.id || 0,
      });
    }
  }, [state, data.gst]);
  return (
    <section className="panel business-card">
      <h3>India regular GST registration</h3>
      {data.gst ? (
        <>
          <p>
            {data.gst.gstin} · {data.gst.state_name} ({data.gst.state_code})
          </p>
          <p>{data.gst.address}</p>
          <p className="form-hint">
            Registration and tax mappings are fixed for these books. GSTIN format checks do not
            verify registration status. Use a separate company file for another registration.
          </p>
        </>
      ) : (
        <>
          <p className="form-hint">
            Configure a single regular GST registration in INR books. Confirm the registration,
            state, tax rates, HSN/SAC and input-credit eligibility independently. This does not
            enable GST Portal filing or IRN generation.
          </p>
          <button
            className="button secondary"
            disabled={busy}
            onClick={() => void mutate('gst_control_accounts', {}, 'GST control accounts ready.')}
          >
            Create standard GST accounts
          </button>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void mutate(
                'configure_gst',
                { gstin, state_code: code, state_name: name, address, output, input },
                'GST registration configured.',
              );
            }}
          >
            <div className="business-grid">
              <Field name="Company GSTIN">
                <input
                  required
                  maxLength={15}
                  value={gstin}
                  onChange={(e) => setGstin(e.target.value.toUpperCase())}
                />
              </Field>
              <Field name="Registration state code">
                <input
                  required
                  maxLength={2}
                  value={code}
                  onChange={(e) => setCode(e.target.value)}
                />
              </Field>
              <Field name="Registration state name">
                <input
                  required
                  maxLength={80}
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                />
              </Field>
              <Field name="Registered postal address">
                <input
                  required
                  maxLength={500}
                  value={address}
                  onChange={(e) => setAddress(e.target.value)}
                />
              </Field>
              {(['cgst', 'sgst', 'igst'] as const).map((k) => (
                <React.Fragment key={k}>
                  <AccountSelect
                    accounts={state.accounts}
                    kinds={['liability']}
                    value={output[k]}
                    change={(v) => setOutput((p) => ({ ...p, [k]: v }))}
                    label={`Output ${k.toUpperCase()} account`}
                  />
                  <AccountSelect
                    accounts={state.accounts}
                    kinds={['asset']}
                    value={input[k]}
                    change={(v) => setInput((p) => ({ ...p, [k]: v }))}
                    label={`Input ${k.toUpperCase()} account`}
                  />
                </React.Fragment>
              ))}
            </div>
            <div className="business-actions">
              <button className="button primary" disabled={busy}>
                Configure GST registration
              </button>
            </div>
          </form>
        </>
      )}
    </section>
  );
}
function GstReport({ state, onError }: { state: Snapshot; onError: (s: string) => void }) {
  const [from, setFrom] = useState(state.company!.fiscal_year_start),
    [to, setTo] = useState(today()),
    [report, setReport] = useState<{
      rows: {
        number: string;
        kind: string;
        date: string;
        event: string;
        sign: number;
        counterparty_gstin: string | null;
        supply_state: string;
        amounts: GstCalculation;
      }[];
      totals: Record<
        string,
        { taxable_minor: string; cgst_minor: string; sgst_minor: string; igst_minor: string }
      >;
    } | null>(null);
  useEffect(() => {
    let live = true;
    void request<typeof report>('gst_report', { from, to })
      .then((v) => {
        if (live) setReport(v);
      })
      .catch((e) => {
        if (live) onError(err(e));
      });
    return () => {
      live = false;
    };
  }, [state, from, to]);
  if (!report) return null;
  return (
    <section className="panel business-card">
      <h3>GST accounting register</h3>
      <p className="form-hint">
        Review worksheet for supported domestic invoices. Accounting reversals are shown on their
        effective date; they are not statutory credit notes. This export is not a GSTR-1/GSTR-3B
        filing payload.
      </p>
      <div className="business-grid">
        <Field name="GST register from">
          <input type="date" required value={from} onChange={(e) => setFrom(e.target.value)} />
        </Field>
        <Field name="GST register through">
          <input type="date" required value={to} onChange={(e) => setTo(e.target.value)} />
        </Field>
      </div>
      <div className="business-actions">
        <button
          className="button secondary"
          onClick={() =>
            download(
              `gst-register-${from}-${to}.csv`,
              csv([
                [
                  'Number',
                  'Kind',
                  'Date',
                  'Event',
                  'GSTIN',
                  'Place of supply',
                  'Line',
                  'HSN / SAC',
                  'Rate basis points',
                  'Taxable minor',
                  'CGST minor',
                  'SGST / UTGST minor',
                  'IGST minor',
                ],
                ...report.rows.flatMap((r) =>
                  r.amounts.lines.map((l, index) => [
                    r.number,
                    r.kind,
                    r.date,
                    r.event,
                    r.counterparty_gstin || '',
                    r.supply_state,
                    index + 1,
                    l.hsn_sac,
                    l.rate_bps,
                    (BigInt(l.taxable_minor) * BigInt(r.sign)).toString(),
                    (BigInt(l.cgst_minor) * BigInt(r.sign)).toString(),
                    (BigInt(l.sgst_minor) * BigInt(r.sign)).toString(),
                    (BigInt(l.igst_minor) * BigInt(r.sign)).toString(),
                  ]),
                ),
              ]),
              'text/csv',
            )
          }
        >
          Export GST register
        </button>
      </div>
      <div className="table-scroll">
        <table>
          <thead>
            <tr>
              <th>Type</th>
              <th>Taxable</th>
              <th>CGST</th>
              <th>SGST / UTGST</th>
              <th>IGST</th>
            </tr>
          </thead>
          <tbody>
            {Object.entries(report.totals).map(([kind, r]) => (
              <tr key={kind}>
                <td>{kind === 'sale' ? 'Outward supplies' : 'Reviewed inward ITC'}</td>
                <td>{money(r.taxable_minor, 'INR')}</td>
                <td>{money(r.cgst_minor, 'INR')}</td>
                <td>{money(r.sgst_minor, 'INR')}</td>
                <td>{money(r.igst_minor, 'INR')}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}

function TallyImporter({ state, busy, mutate, onError }: Props) {
  const [contents, setContents] = useState(''),
    [source, setSource] = useState(''),
    [confirmed, setConfirmed] = useState(false),
    [inspection, setInspection] = useState<{
      ledgers: string[];
      vouchers: { guid: string; date: string; number: string; kind: string }[];
    } | null>(null),
    [maps, setMaps] = useState<Record<string, number>>({}),
    [preview, setPreview] = useState<{
      count: number;
      trial_balance: { debit_minor: string; credit_minor: string; balanced: boolean };
    } | null>(null);
  const payload = { contents, source_id: source, currency: 'INR', accounts: maps, preview: true };
  return (
    <section className="panel business-card">
      <h3>Experimental Tally XML general-voucher import</h3>
      <p className="form-hint">
        Supports only simple Journal, Receipt, Payment and Contra vouchers with GUIDs and no
        allocations. Invoice, stock, GST, master, optional/cancelled and foreign-currency structures
        are rejected. Validate on a copy of books and compare source totals. Compatibility with your
        Tally version requires a representative export.
      </p>
      <Field name="Tally voucher XML">
        <textarea
          rows={7}
          value={contents}
          onChange={(e) => {
            setContents(e.target.value);
            setInspection(null);
            setPreview(null);
          }}
        />
      </Field>
      <Field name="Load Tally XML file">
        <input
          type="file"
          accept=".xml"
          onChange={async (e) => {
            const f = e.target.files?.[0];
            if (!f) return;
            if (f.size > 20 * 1024 * 1024) {
              onError('Tally XML is limited to 20 MiB.');
              return;
            }
            setContents(await f.text());
            setInspection(null);
            setPreview(null);
          }}
        />
      </Field>
      <button
        className="button secondary"
        disabled={busy}
        onClick={async () => {
          try {
            setInspection(await request('inspect_tally', { contents }));
            setMaps({});
            setPreview(null);
          } catch (e) {
            onError(err(e));
          }
        }}
      >
        Inspect Tally XML
      </button>
      {inspection && (
        <>
          <p className="form-hint">
            {inspection.vouchers.length} supported vouchers · {inspection.ledgers.length} ledgers.
            Explicitly map every source ledger.
          </p>
          <div className="business-grid">
            {inspection.ledgers.map((name) => (
              <AccountSelect
                key={name}
                accounts={state.accounts}
                kinds={['asset', 'liability', 'equity', 'income', 'expense']}
                value={maps[name] || 0}
                change={(id) => {
                  setMaps((p) => ({ ...p, [name]: id }));
                  setPreview(null);
                }}
                label={`Map Tally ledger: ${name}`}
              />
            ))}
            <Field name="Stable source company ID">
              <input
                required
                maxLength={30}
                value={source}
                onChange={(e) => {
                  setSource(e.target.value);
                  setPreview(null);
                }}
              />
            </Field>
          </div>
          <label className="checkbox-label">
            <input
              type="checkbox"
              checked={confirmed}
              onChange={(e) => setConfirmed(e.target.checked)}
            />
            I confirmed the source and destination books use INR.
          </label>
          <div className="business-actions">
            <button
              className="button secondary"
              disabled={busy || !confirmed || !source || inspection.ledgers.some((l) => !maps[l])}
              onClick={async () => {
                try {
                  setPreview(await request('import_tally', payload));
                } catch (e) {
                  onError(err(e));
                }
              }}
            >
              Preview mapped Tally import
            </button>
            {preview && (
              <>
                <span>
                  {preview.count} vouchers validated · Trial balance{' '}
                  {money(preview.trial_balance.debit_minor, 'INR')} /{' '}
                  {money(preview.trial_balance.credit_minor, 'INR')}
                </span>
                <button
                  className="button primary"
                  disabled={busy || !confirmed}
                  onClick={async () => {
                    if (
                      await mutate(
                        'import_tally',
                        { ...payload, preview: false },
                        'Tally general vouchers imported.',
                      )
                    ) {
                      setPreview(null);
                      setInspection(null);
                      setContents('');
                    }
                  }}
                >
                  Import mapped Tally vouchers
                </button>
              </>
            )}
          </div>
        </>
      )}
    </section>
  );
}
