// Native WebDriver smoke tests. Run against an already-built executable on Linux/Windows.
import { spawn } from 'node:child_process';
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import assert from 'node:assert/strict';
if (process.platform === 'darwin')
  throw new Error(
    'The free tauri-driver has no macOS WKWebView driver. Run browser WebKit tests and the documented native manual checks.',
  );
const directory = await mkdtemp(join(tmpdir(), 'open-ledger-native-'));
const executable = resolve(
  process.env.OPEN_LEDGER_BINARY ||
    join(
      process.env.CARGO_TARGET_DIR || 'target',
      'release',
      process.platform === 'win32' ? 'open-ledger-desktop.exe' : 'open-ledger-desktop',
    ),
);
const driver = spawn('tauri-driver', ['--port', '4444'], {
  stdio: 'inherit',
  env: { ...process.env, OPEN_LEDGER_DATA_DIR: directory },
});
let session;
async function send(path, method = 'POST', body) {
  const r = await fetch(`http://127.0.0.1:4444${path}`, {
    method,
    headers: { 'Content-Type': 'application/json' },
    ...(body === undefined ? {} : { body: JSON.stringify(body) }),
  });
  const json = await r.json();
  if (!r.ok || json.value?.error) throw new Error(JSON.stringify(json));
  return json.value;
}
async function poll(fn, timeout = 30000) {
  const start = Date.now();
  let error;
  while (Date.now() - start < timeout) {
    try {
      return await fn();
    } catch (e) {
      error = e;
      await new Promise((r) => setTimeout(r, 100));
    }
  }
  throw error;
}
async function script(code, args = []) {
  return send(`/session/${session}/execute/sync`, 'POST', { script: code, args });
}
async function invoke(command, payload = {}) {
  return send(`/session/${session}/execute/async`, 'POST', {
    script:
      'const done=arguments[arguments.length-1];window.__TAURI__.core.invoke("ledger_request",{command:arguments[0],payload:arguments[1]}).then(v=>done({ok:true,value:v}),e=>done({ok:false,error:String(e)}));',
    args: [command, payload],
  }).then((r) => {
    assert.equal(r.ok, true, r.error);
    return r.value;
  });
}
try {
  const created = await poll(() =>
    send('/session', 'POST', {
      capabilities: { alwaysMatch: { 'tauri:options': { application: executable } } },
    }),
  );
  session = created.sessionId;
  await send(`/session/${session}/timeouts`, 'POST', { pageLoad: 30000, script: 10000 });
  await poll(async () => {
    assert.equal(
      await script(
        "return Boolean(window.__TAURI__?.core) && location.href !== 'about:blank' && document.querySelector('h2')?.textContent === 'Create your company'",
      ),
      true,
    );
  });
  await invoke('create_company', {
    name: 'Native smoke books',
    currency: 'INR',
    fiscal_year_start: '2026-01-01',
  });
  const id = await invoke('post_journal', {
    date: '2026-10-01',
    reference: 'NATIVE',
    description: 'Native capital',
    request_id: 'native-capital',
    lines: [
      { account_id: 2, side: 'debit', amount: '1000' },
      { account_id: 6, side: 'credit', amount: '1000' },
    ],
  });
  await send(`/session/${session}/refresh`, 'POST', {});
  await poll(async () => {
    assert.equal(await script('return document.querySelector("h1")?.textContent'), 'Overview');
  });
  await script('document.querySelectorAll("nav button")[5].click();return true');
  await poll(async () => {
    assert.equal(
      await script('return document.querySelector("h1")?.textContent'),
      'Account ledger',
    );
  });
  const ledger = await invoke('ledger', { account_id: 2, from: '2026-10-01', to: '2026-10-02' });
  assert.equal(ledger.closing_minor, '100000');
  const backup = await invoke('backup');
  await invoke('reverse_journal', {
    id: id.id,
    date: '2026-10-02',
    reason: 'Native reversal',
    request_id: 'native-reversal',
  });
  assert.equal((await invoke('trial_balance', { as_of: '2026-10-02' })).debit_minor, '0');
  const preview = await invoke('preview_backup', { contents: backup });
  assert.equal(preview.journal_count, 1);
  const restored = await invoke('restore_backup', { contents: backup });
  assert.equal(restored.recovery_saved, true);
  assert.equal((await invoke('ledger', { account_id: 2 })).closing_minor, '100000');
  const failed = await send(`/session/${session}/execute/async`, 'POST', {
    script:
      'const done=arguments[arguments.length-1];window.__TAURI__.core.invoke("ledger_request",{command:"restore_backup",payload:{contents:"{broken"}}).then(()=>done(false),()=>done(true));',
    args: [],
  });
  assert.equal(failed, true);
  assert.equal((await invoke('state')).journal_count, 1);
  const customer = await invoke('create_party', {
    name: 'Native return customer',
    kind: 'customer',
    contact: '',
    account_id: 3,
  });
  const invoiceInput = {
    kind: 'sale',
    date: '2026-10-01',
    due_date: '2026-10-02',
    party_id: customer.id,
    invoice_id: null,
    money_account_id: null,
    amount: null,
    note: 'Native return source',
    lines: [
      { description: 'Service', quantity: '3', unit_price: '10', account_id: 7, item_id: null },
    ],
    request_id: 'native-invoice',
  };
  const invoice = await invoke('post_document', invoiceInput);
  await invoke('post_document', {
    ...invoiceInput,
    kind: 'receipt',
    date: '2026-10-02',
    due_date: null,
    invoice_id: invoice.id,
    money_account_id: 2,
    amount: '30',
    lines: [],
    request_id: 'native-receipt',
  });
  const note = {
    ...invoiceInput,
    kind: 'credit_note',
    date: '2026-10-03',
    due_date: null,
    invoice_id: invoice.id,
    lines: [],
    return_lines: [{ line: 0, quantity: '3' }],
    request_id: 'native-return',
    note: 'Full native cancellation',
  };
  const returnPreview = await invoke('preview_document', note);
  assert.equal(returnPreview.document.total_minor, '3000');
  assert.equal((await invoke('state')).journal_count, 3);
  await invoke('post_document', note);
  await invoke('post_document', {
    ...invoiceInput,
    kind: 'customer_refund',
    date: '2026-10-03',
    due_date: null,
    invoice_id: invoice.id,
    money_account_id: 2,
    amount: '30',
    lines: [],
    request_id: 'native-refund',
  });
  assert.equal(
    (await invoke('ledger', { account_id: 3, from: null, to: null })).closing_minor,
    '0',
  );
  const returnsBackup = await invoke('backup');
  await invoke('restore_backup', { contents: returnsBackup });
  const business = await invoke('business', { offset: 0, limit: 50, as_of: null });
  const returned = business.documents.find((d) => d.id === invoice.id);
  assert.equal(returned.credited_minor, '3000');
  assert.equal(returned.outstanding_minor, '0');
  assert.equal((await invoke('returnable_invoice', { id: invoice.id })).remaining_milli[0], '0');
  const controls = await invoke('gst_control_accounts');
  await invoke('configure_gst', {
    gstin: '29ABCDE1234F1Z5',
    state_code: '29',
    state_name: 'Karnataka',
    address: 'Synthetic native test office',
    ...controls,
  });
  const gst = {
    supply_state: '29',
    supply_state_name: 'Karnataka',
    counterparty_gstin: null,
    counterparty_address: 'Synthetic recipient office',
    delivery_address: 'Synthetic service address',
    supplier_reference: null,
    credit_reviewed: false,
    rates_bps: [1800],
    hsn_sac: ['998314'],
  };
  await invoke('post_document', {
    ...invoiceInput,
    date: '2026-10-04',
    due_date: '2026-10-04',
    gst,
    lines: [
      {
        description: 'Taxable service',
        quantity: '1',
        unit_price: '100',
        account_id: 7,
        item_id: null,
      },
    ],
    request_id: 'native-taxable',
  });
  await invoke('post_document', {
    ...invoiceInput,
    date: '2026-10-04',
    due_date: '2026-10-04',
    gst: {
      ...gst,
      rates_bps: [0],
      classification: { kind: 'exempt', reason: 'Synthetic reviewed exemption basis' },
    },
    lines: [
      {
        description: 'Exempt service',
        quantity: '1',
        unit_price: '50',
        account_id: 7,
        item_id: null,
      },
    ],
    request_id: 'native-exempt',
  });
  const taxSupplier = await invoke('create_party', {
    name: 'Native ITC supplier',
    kind: 'supplier',
    contact: '',
    account_id: 5,
  });
  await invoke('post_document', {
    ...invoiceInput,
    kind: 'purchase',
    party_id: taxSupplier.id,
    date: '2026-10-04',
    due_date: '2026-10-04',
    gst: {
      ...gst,
      counterparty_gstin: '29ABCDE1234F1Z5',
      supplier_reference: 'NATIVE-ITC',
      credit_reviewed: true,
      itc_allocation: {
        eligible_bps: [5000],
        reason: 'Synthetic reviewed allocation',
        return_treatment: 'permanent_reversal',
      },
    },
    lines: [
      {
        description: 'Apportioned purchase',
        quantity: '1',
        unit_price: '100',
        account_id: 8,
        item_id: null,
      },
    ],
    request_id: 'native-apportioned',
  });
  const taxDraft = await invoke('gst_return_draft', { period: '2026-10', frequency: 'monthly' });
  assert.equal(taxDraft.book_recoverable_input_tax.cgst_minor, '450');
  assert.equal(taxDraft.gstr3b_book_3_1_a.taxable_minor, '10000');
  assert.equal(taxDraft.gstr3b_book_3_1_c.taxable_minor, '5000');
  assert.equal(taxDraft.gstr1_sections.exemp.taxable_minor, '5000');
  assert.equal(taxDraft.reconciled, true);
  assert.equal(taxDraft.filing_ready, false);
  const classifiedBackup = await invoke('backup');
  assert.equal(JSON.parse(classifiedBackup).version, 6);
  await invoke('restore_backup', { contents: classifiedBackup });
  assert.deepEqual(
    await invoke('gst_return_draft', { period: '2026-10', frequency: 'monthly' }),
    taxDraft,
  );
  console.log(
    'Native IPC, rendered reports, posting, reversal, backup, restore, malformed restore, linked returns/refunds and GST classification/draft/reconciliation checks passed.',
  );
} catch (error) {
  if (session) {
    const diagnostics = await script(
      `const probe = f => {try {return f()} catch(e) {return String(e)}};
       return {url:location.href,title:document.title,html:document.documentElement.outerHTML,
       storage:probe(()=>localStorage.getItem('open-ledger-locale')),
       intl:probe(()=>new Intl.NumberFormat(navigator.language,{style:'currency',currency:'INR'}).formatToParts(0n)),
       tauri:Boolean(window.__TAURI__?.core),resources:performance.getEntriesByType('resource').map(r=>({name:r.name,size:r.transferSize}))}`,
    ).catch((e) => ({ error: String(e) }));
    console.error('Native page diagnostics:', JSON.stringify(diagnostics));
    const screenshot = await send(`/session/${session}/screenshot`, 'GET').catch(() => null);
    if (typeof screenshot === 'string') {
      await mkdir('test-results', { recursive: true });
      await writeFile('test-results/native.png', Buffer.from(screenshot, 'base64'));
    }
  }
  throw error;
} finally {
  if (session) await send(`/session/${session}`, 'DELETE').catch(() => {});
  driver.kill();
  await rm(directory, { recursive: true, force: true, maxRetries: 20, retryDelay: 100 });
}
