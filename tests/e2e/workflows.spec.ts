import { readFileSync } from 'node:fs';
import AxeBuilder from '@axe-core/playwright';
import { test, expect, Page, APIRequestContext } from '@playwright/test';
async function api(request: APIRequestContext, command: string, payload: object = {}) {
  const response = await request.post(`/api/${command}`, { data: payload });
  expect(response.ok(), await response.text()).toBeTruthy();
  return response.json();
}
async function company(page: Page, request: APIRequestContext, name: string) {
  await api(request, 'new_books', { name, currency: 'INR', fiscal_year_start: '2026-01-01' });
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Overview', exact: true })).toBeVisible();
}
async function nav(page: Page, name: string) {
  await page
    .locator('nav')
    .getByRole('button', { name: new RegExp(`^${name}(?: \\d+)?$`) })
    .click();
}
async function journal(request: APIRequestContext, key: string, amount = '100') {
  return api(request, 'post_journal', {
    date: '2026-10-01',
    reference: key,
    description: 'Opening capital',
    request_id: key,
    lines: [
      { account_id: 2, side: 'debit', amount },
      { account_id: 6, side: 'credit', amount },
    ],
  });
}
test('linked invoice cancellation, reviewed return and customer refund work through the interface', async ({
  page,
  request,
}, testInfo) => {
  await company(page, request, 'Returns UI');
  const party = await api(request, 'create_party', {
    name: 'Return customer',
    kind: 'customer',
    contact: '',
    account_id: 3,
  });
  const base = {
    date: '2026-10-01',
    due_date: '2026-10-02',
    party_id: party.id,
    invoice_id: null,
    money_account_id: null,
    amount: null,
    note: 'Returnable service',
    request_id: 'return-source',
    kind: 'sale',
    lines: [
      {
        description: 'Service units',
        quantity: '3',
        unit_price: '100',
        account_id: 7,
        item_id: null,
      },
    ],
  };
  const invoice = await api(request, 'post_document', base);
  await api(request, 'post_document', {
    ...base,
    kind: 'receipt',
    date: '2026-10-02',
    due_date: null,
    invoice_id: invoice.id,
    money_account_id: 2,
    amount: '300',
    lines: [],
    request_id: 'paid',
  });
  await nav(page, 'Invoices & settlements');
  await page.getByRole('button', { name: 'Return / cancel', exact: true }).click();
  await page.getByLabel('Return reason').fill('Cancel the remaining service');
  await page.getByRole('button', { name: 'Cancel remaining invoice', exact: true }).click();
  await expect(page.getByLabel(/Return quantity 1/)).toHaveValue('3.000');
  await page.getByRole('button', { name: 'Review return', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Review sales credit note' })).toBeVisible();
  await expect(page.getByLabel('Invoice return')).toContainText('₹300.00');
  await expect(
    page.getByRole('heading', { name: 'Return / cancel INV-000001', exact: true }),
  ).toBeFocused();
  // Safari on macOS follows the system's keyboard-navigation preference;
  // Option-Tab includes buttons when ordinary Tab is restricted to fields.
  const nextControl =
    testInfo.project.name === 'webkit' && process.platform === 'darwin' ? 'Alt+Tab' : 'Tab';
  await page.keyboard.press(nextControl);
  await page.keyboard.press(nextControl);
  await page.keyboard.press(nextControl);
  await expect(page.getByRole('button', { name: 'Post return note', exact: true })).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('button', { name: 'CN-000001', exact: true })).toBeVisible();
  const row = page
    .getByRole('row')
    .filter({ has: page.getByRole('button', { name: 'INV-000001', exact: true }) });
  await expect(row).toContainText('Refund due');
  await page.getByRole('button', { name: 'Record refund', exact: true }).click();
  await page.getByLabel('Cash / bank account').selectOption('2');
  await expect(page.getByLabel('Amount', { exact: true })).toHaveValue('300.00');
  await page.getByRole('button', { name: 'Review document' }).click();
  await page.getByRole('button', { name: 'Post customer refund', exact: true }).click();
  await expect(row).toContainText('Fully credited');
  await page.getByRole('button', { name: 'CN-000001', exact: true }).click();
  await expect(page.getByText('Linked original invoice:', { exact: false }).last()).toContainText(
    'INV-000001',
  );
  const downloading = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Export document (HTML)' }).click();
  const exported = await downloading;
  const path = await exported.path();
  const text = readFileSync(path!, 'utf8');
  expect(text).toContain('Service units');
  expect(text).toContain('Linked original invoice: INV-000001');
  const ledger = await api(request, 'ledger', { account_id: 2, from: null, to: null });
  expect(ledger.closing_minor).toBe('0');
});

test('stock return preview exposes original issue cost and preserves inventory after restore', async ({
  page,
  request,
}) => {
  await company(page, request, 'Stock returns UI');
  const supplier = await api(request, 'create_party', {
    name: 'Stock supplier',
    kind: 'supplier',
    contact: '',
    account_id: 5,
  });
  const customer = await api(request, 'create_party', {
    name: 'Stock customer',
    kind: 'customer',
    contact: '',
    account_id: 3,
  });
  const asset = await api(request, 'create_account', {
    code: '1300',
    name: 'Inventory',
    kind: 'asset',
  });
  const item = await api(request, 'create_item', {
    code: 'W',
    name: 'Widget',
    unit: 'pcs',
    asset_account_id: asset.id,
    expense_account_id: 8,
  });
  const base = {
    date: '2026-10-01',
    due_date: '2026-10-02',
    party_id: supplier.id,
    invoice_id: null,
    money_account_id: null,
    amount: null,
    note: 'Stock purchase',
    request_id: 'stock-in',
    kind: 'purchase',
    lines: [
      {
        description: 'Widget',
        quantity: '10',
        unit_price: '10',
        account_id: asset.id,
        item_id: item.id,
      },
    ],
  };
  await api(request, 'post_document', base);
  await api(request, 'post_document', {
    ...base,
    party_id: customer.id,
    kind: 'sale',
    request_id: 'stock-sale',
    lines: [{ ...base.lines[0], quantity: '4', unit_price: '30', account_id: 7 }],
  });
  await nav(page, 'Invoices & settlements');
  const saleRow = page
    .getByRole('row')
    .filter({ has: page.getByRole('button', { name: 'INV-000001', exact: true }) });
  await saleRow.getByRole('button', { name: 'Return / cancel', exact: true }).click();
  await page.getByLabel('Return reason').fill('Two units returned');
  await page.getByLabel(/Return quantity 1/).fill('2');
  await page.getByRole('button', { name: 'Review return', exact: true }).click();
  await expect(page.getByLabel('Invoice return')).toContainText('₹20.00');
  await page.getByRole('button', { name: 'Post return note', exact: true }).click();
  await expect(saleRow).toContainText('60.00');
  const b = await api(request, 'business', { offset: 0, limit: 50, as_of: null });
  expect(b.stock[0].quantity_milli).toBe('8000');
  expect(b.stock[0].value_minor).toBe('8000');
  const backup = await api(request, 'backup');
  await api(request, 'restore_backup', { contents: backup });
  const restored = await api(request, 'business', { offset: 0, limit: 50, as_of: null });
  expect(restored.stock).toEqual(b.stock);
});

test('invalid native locales and unavailable preference storage preserve access to books', async ({
  page,
  request,
}) => {
  await page.addInitScript(() => {
    Object.defineProperty(navigator, 'language', { get: () => 'C' });
    try {
      localStorage.setItem('open-ledger-locale', 'C.UTF-8');
    } catch {
      // A later part of this test intentionally disables optional storage.
    }
  });
  await company(page, request, 'POSIX locale regression');
  await expect(page.locator('.metric').first()).toContainText('₹0.00');
  await nav(page, 'Company & periods');
  await expect(page.getByLabel('Number and date locale')).toHaveValue('en-IN');
  await page.addInitScript(() => {
    Object.defineProperty(window, 'localStorage', {
      get: () => {
        throw new DOMException('Storage unavailable', 'SecurityError');
      },
    });
  });
  await page.reload();
  await expect(page.getByRole('heading', { name: 'Overview', exact: true })).toBeVisible();
  await expect(page.locator('.metric').first()).toContainText('₹0.00');
  await nav(page, 'Company & periods');
  await expect(page.getByLabel('Number and date locale')).toHaveValue('en-IN');
  await api(request, 'create_party', {
    name: 'No-storage customer',
    kind: 'customer',
    contact: '',
    account_id: 3,
  });
  await nav(page, 'Invoices & settlements');
  await page.getByRole('button', { name: 'New sales invoice' }).click();
  await page.getByLabel('Customer', { exact: true }).selectOption({ label: 'No-storage customer' });
  await page.getByLabel('Description', { exact: true }).fill('Works without browser storage');
  await page.getByLabel('Unit price', { exact: true }).fill('10');
  await page.getByLabel('Posting account').selectOption('7');
  await page.getByRole('button', { name: 'Save draft on this device' }).click();
  await expect(page.getByRole('alert').last()).toContainText('Draft could not be saved');
  await page.getByRole('button', { name: 'Review document' }).click();
  await page.getByRole('button', { name: 'Post sales invoice' }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'INV-000001', exact: true })).toBeVisible();
});

test('customer invoice and partial receipt through the interface', async ({ page, request }) => {
  await company(page, request, 'Invoice UI');
  await nav(page, 'Invoices & settlements');
  await page.getByRole('button', { name: 'Customers & suppliers', exact: true }).click();
  await page.getByLabel('Name', { exact: true }).fill('Northstar');
  await page.getByLabel('Contact / address').fill('buyer@example.test');
  await page.getByLabel('Receivable account').selectOption('3');
  await page.getByRole('button', { name: 'Add party', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('Party added');
  await page.getByRole('button', { name: 'New sales invoice' }).click();
  await page.getByLabel('Customer', { exact: true }).selectOption({ label: 'Northstar' });
  await page.getByLabel('Description', { exact: true }).fill('Design services');
  await page.getByLabel('Unit price', { exact: true }).fill('1000');
  await page.getByLabel('Posting account').selectOption('7');
  await page.getByRole('button', { name: 'Review document' }).click();
  await page.getByRole('button', { name: 'Post sales invoice' }).click();
  await expect(page.getByRole('button', { name: 'INV-000001', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Record receipt', exact: true }).click();
  await page.getByLabel('Cash / bank account').selectOption('2');
  await page.getByLabel('Amount', { exact: true }).fill('400');
  await page.getByRole('button', { name: 'Review document' }).click();
  await page.getByRole('button', { name: 'Post receipt', exact: true }).click();
  const row = page
    .getByRole('row')
    .filter({ has: page.getByRole('button', { name: 'INV-000001', exact: true }) });
  await expect(row).toContainText('600.00');
  await page.getByRole('button', { name: 'INV-000001', exact: true }).click();
  const download = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Export document (HTML)' }).click();
  expect((await download).suggestedFilename()).toBe('INV-000001.html');
  await nav(page, 'Financial statements');
  await expect(page.getByRole('heading', { name: 'Balance sheet', exact: true })).toBeVisible();
  await expect(page.getByText(/Balanced$/)).toBeVisible();
});
test('posting, filtered reports, reversal, backup and restore preserve the books', async ({
  page,
  request,
}) => {
  await company(page, request, 'Recovery UI');
  await page.getByRole('button', { name: /New journal/ }).click();
  await page.getByLabel('Description', { exact: true }).fill('Capital via interface');
  const selects = page.getByRole('dialog').locator('.posting-row select');
  await selects.nth(0).selectOption('2');
  await selects.nth(2).selectOption('6');
  const amounts = page.getByRole('dialog').locator('.posting-row input');
  await amounts.nth(0).fill('100');
  await amounts.nth(1).fill('100');
  await page.getByRole('button', { name: /Post journal/ }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await nav(page, 'Account ledger');
  await page.getByLabel('Through date').fill('2026-12-31');
  await expect(page.getByText('Capital via interface')).toBeVisible();
  const backup = await api(request, 'backup');
  await journal(request, 'extra', '50');
  await api(request, 'restore_backup', { contents: backup });
  await page.reload();
  await nav(page, 'Trial balance');
  await expect(page.getByText(/Debits equal credits/)).toBeVisible();
  const state = await api(request, 'state');
  expect(state.journal_count).toBe(1);
  await nav(page, 'Journal entries');
  await page.getByRole('button', { name: state.journals[0].reference, exact: true }).click();
  await page.getByRole('button', { name: /Reverse entry/ }).click();
  await page.getByLabel('Reason').fill('Correction');
  await page.getByRole('button', { name: /Post reversal/ }).click();
  expect((await api(request, 'state')).journal_count).toBe(2);
  const failed = await request.post('/api/restore_backup', { data: { contents: '{broken' } });
  expect(failed.ok()).toBe(false);
  expect((await api(request, 'state')).journal_count).toBe(2);
});
test('keyboard focus, pagination, and locale display remain usable', async ({ page, request }) => {
  await company(page, request, 'Keyboard UI');
  for (let i = 0; i < 55; i++) await journal(request, `page-${i}`, '0.01');
  await page.reload();
  await nav(page, 'Journal entries');
  await expect(page.getByText('Page 1', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Next', exact: true }).click();
  await expect(page.getByText('Page 2', { exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'page-0', exact: true })).toBeVisible();
  await page.keyboard.press(process.platform === 'darwin' ? 'Meta+j' : 'Control+j');
  await expect(page.getByRole('dialog')).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await nav(page, 'Company & periods');
  await page.getByLabel('Number and date locale').selectOption('en-IN');
  await expect(page.getByRole('heading', { name: 'Overview', exact: true })).toBeVisible();
});
test('core pages, invoice forms and journal dialog pass automated accessibility checks', async ({
  page,
  request,
}) => {
  await company(page, request, 'Accessibility checks');
  const check = async (name: string) => {
    const results = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'])
      .analyze();
    expect(
      results.violations.map((v) => ({
        id: v.id,
        impact: v.impact,
        nodes: v.nodes.map((n) => ({ target: n.target, summary: n.failureSummary })),
      })),
      name,
    ).toEqual([]);
  };
  await check('Overview');
  for (const name of [
    'Journal entries',
    'Chart of accounts',
    'Account ledger',
    'Trial balance',
    'Invoices & settlements',
    'Inventory',
    'Financial statements',
    'Bank reconciliation',
    'Company & periods',
    'Backups',
  ]) {
    await nav(page, name);
    await check(name);
  }
  await nav(page, 'Invoices & settlements');
  await page.getByRole('button', { name: 'New sales invoice' }).click();
  await check('Invoice form');
  await page.keyboard.press('Escape');
  await page.keyboard.press(process.platform === 'darwin' ? 'Meta+j' : 'Control+j');
  await expect(page.getByRole('dialog')).toBeVisible();
  await check('Journal dialog');
  await page.keyboard.press('Escape');
  await api(request, 'new_books', {});
  await page.reload();
  await expect(page.getByRole('heading', { name: 'Create your company' })).toBeVisible();
  await check('Welcome form');
  await api(request, 'create_demo');
  await page.reload();
  await expect(page.getByRole('heading', { name: 'Overview', exact: true })).toBeVisible();
  await check('Populated books');
});
test('narrow reflow, keyboard navigation and forced colors preserve access to core workflows', async ({
  page,
  request,
}, testInfo) => {
  await company(page, request, 'Reflow checks');
  await page.setViewportSize({ width: 320, height: 800 });
  await page.emulateMedia({ forcedColors: 'active', reducedMotion: 'reduce' });
  for (const name of [
    'Invoices & settlements',
    'Inventory',
    'Financial statements',
    'Bank reconciliation',
    'Company & periods',
    'Backups',
  ]) {
    const toggle = page.getByRole('button', { name: 'Toggle navigation' });
    await toggle.focus();
    await page.keyboard.press('Enter');
    await expect(toggle).toHaveAttribute('aria-expanded', 'true');
    await nav(page, name);
    await expect(toggle).toHaveAttribute('aria-expanded', 'false');
    await expect(page.locator('h1')).toBeFocused();
    expect(
      await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1),
      name,
    ).toBe(true);
  }
  await page.getByRole('button', { name: 'Toggle navigation' }).click();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('button', { name: 'Toggle navigation' })).toBeFocused();
  await page.screenshot({ path: testInfo.outputPath('forced-colors-reflow.png'), fullPage: true });
});
test('loopback API rejects foreign origins', async ({ request }) => {
  const response = await request.post('/api/state', {
    headers: { Origin: 'https://attacker.example' },
    data: {},
  });
  expect(response.status()).toBe(403);
});

test('inventory adjustment and stock sale calculate cost through the interface', async ({
  page,
  request,
}) => {
  await company(page, request, 'Inventory UI');
  const asset = await api(request, 'create_account', {
    code: '1300',
    name: 'Inventory asset',
    kind: 'asset',
  });
  const expense = await api(request, 'create_account', {
    code: '5100',
    name: 'Cost of sales',
    kind: 'expense',
  });
  await api(request, 'create_item', {
    code: 'WIDGET',
    name: 'Widget',
    unit: 'each',
    asset_account_id: asset.id,
    expense_account_id: expense.id,
  });
  await api(request, 'create_party', {
    name: 'Buyer',
    kind: 'customer',
    contact: '',
    account_id: 3,
  });
  await page.reload();
  await nav(page, 'Inventory');
  await page.getByRole('button', { name: 'Stock adjustment', exact: true }).click();
  await page
    .getByLabel('Inventory item', { exact: true })
    .selectOption({ label: 'WIDGET · Widget (each)' });
  await page.getByLabel('Quantity (up to 3 decimals)').fill('2');
  await page.getByLabel('Unit price', { exact: true }).fill('10');
  await page.getByLabel('Offset account').selectOption('6');
  await page.getByRole('button', { name: 'Review document' }).click();
  await page.getByRole('button', { name: 'Post stock adjustment', exact: true }).click();
  await expect(page.getByRole('cell', { name: '20.00', exact: false }).first()).toBeVisible();
  await nav(page, 'Invoices & settlements');
  await page.getByRole('button', { name: 'New sales invoice' }).click();
  await page.getByLabel('Customer', { exact: true }).selectOption({ label: 'Buyer' });
  await page
    .getByLabel('Inventory item', { exact: true })
    .selectOption({ label: 'WIDGET · Widget (each)' });
  await page.getByLabel('Unit price', { exact: true }).fill('15');
  await page.getByLabel('Posting account').selectOption('7');
  await page.getByRole('button', { name: 'Review document' }).click();
  await page.getByRole('button', { name: 'Post sales invoice' }).click();
  await expect(page.getByRole('button', { name: 'INV-000001', exact: true })).toBeVisible();
  const report = await api(request, 'financial_statements');
  expect(report.profit_minor).toBe('500');
  const stock = await api(request, 'business');
  expect(stock.stock[0].quantity_milli).toBe('1000');
  expect(stock.stock[0].value_minor).toBe('1000');
});
test('bank reconciliation and empty-earnings period closing work through the interface', async ({
  page,
  request,
}) => {
  await company(page, request, 'Bank and closing UI');
  await journal(request, 'bank-capital', '42');
  await page.reload();
  await nav(page, 'Bank reconciliation');
  await page.getByLabel('Statement closing').fill('42');
  await page.getByLabel('Statement reference').fill('Test statement');
  await page.getByRole('checkbox', { name: /Clear bank-capital line/ }).check();
  await page.getByRole('button', { name: 'Save reconciliation' }).click();
  await expect(page.getByRole('cell', { name: 'Test statement', exact: true })).toBeVisible();
  await nav(page, 'Company & periods');
  await page.getByLabel('Period ends').fill('2026-10-14');
  await page.getByLabel('Retained earnings / equity account').selectOption('6');
  await page.getByRole('checkbox', { name: /I reviewed the period/ }).check();
  await page.getByRole('button', { name: 'Close period', exact: true }).click();
  await expect(
    page.getByRole('cell', { name: 'No earnings to transfer', exact: true }),
  ).toBeVisible();
  const failed = await request.post('/api/post_journal', {
    data: {
      date: '2026-10-01',
      reference: 'closed',
      description: 'Blocked after close',
      request_id: 'closed',
      lines: [
        { account_id: 2, side: 'debit', amount: '1' },
        { account_id: 6, side: 'credit', amount: '1' },
      ],
    },
  });
  expect(failed.ok()).toBe(false);
});

test('India GST setup, invoice tax and register work through the interface', async ({
  page,
  request,
}) => {
  await company(page, request, 'India GST UI');
  await nav(page, 'Company & periods');
  await page.getByRole('button', { name: 'Create standard GST accounts', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('GST control accounts ready');
  await page.getByLabel('Company GSTIN').fill('29ABCDE1234F1Z5');
  await page.getByLabel('Registration state code').fill('29');
  await page.getByLabel('Registration state name').fill('Karnataka');
  await page.getByLabel('Registered postal address').fill('Synthetic registered office, Bengaluru');
  await page.getByRole('button', { name: 'Configure GST registration', exact: true }).click();
  await expect(page.getByText('29ABCDE1234F1Z5 · Karnataka (29)', { exact: true })).toBeVisible();
  await api(request, 'create_party', {
    name: 'GST buyer',
    kind: 'customer',
    contact: '',
    account_id: 3,
  });
  await page.reload();
  await nav(page, 'Invoices & settlements');
  await page.getByRole('button', { name: 'New sales invoice' }).click();
  await page.getByLabel('Customer', { exact: true }).selectOption({ label: 'GST buyer' });
  await page.getByLabel('Description', { exact: true }).fill('Synthetic service');
  await page.getByLabel('Unit price', { exact: true }).fill('1000');
  await page.getByLabel('Posting account').selectOption('7');
  await page.getByRole('checkbox', { name: 'Record India tax details' }).check();
  await page.getByLabel('Counterparty postal address').fill('Synthetic recipient address');
  await page.getByLabel('Delivery address').fill('Synthetic service delivery address');
  await page.getByLabel('Reviewed GST rate (%)').fill('18');
  await page.getByLabel('HSN / SAC').fill('998314');
  await page.getByRole('button', { name: 'Review document' }).click();
  await expect(page.getByText('Total: ₹1,180.00', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Post sales invoice' }).click();
  await expect(page.getByRole('button', { name: 'INV-000001', exact: true })).toBeVisible();
  const b = await api(request, 'business');
  expect(b.documents[0].gst.cgst_minor).toBe('9000');
  expect(b.documents[0].gst.sgst_minor).toBe('9000');
  await nav(page, 'Financial statements');
  await expect(
    page.getByRole('heading', { name: 'GST accounting register', exact: true }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Prepare GST return draft', exact: true }).click();
  await expect(
    page.getByRole('status').filter({ hasText: 'GST control activity matches' }),
  ).toBeVisible();
  await expect(
    page.getByRole('heading', { name: 'GSTR-1 candidate sections', exact: true }),
  ).toBeVisible();
  const draftDownload = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Export complete GST draft (JSON)', exact: true }).click();
  const draftFile = await draftDownload;
  const draft = JSON.parse(readFileSync((await draftFile.path())!, 'utf8'));
  expect(draft.format).toBe('open-ledger-gst-return-draft');
  expect(draft.filing_ready).toBe(false);
  expect(draft.reconciled).toBe(true);
  expect(draft.gstr1_sections.b2cs.cgst_minor).toBe('9000');
  expect(
    draft.reconciliation.every((r: { difference_minor: string }) => r.difference_minor === '0'),
  ).toBe(true);
  const csvDownload = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Export draft B2CS (1)', exact: true }).click();
  const csvFile = await csvDownload;
  expect(csvFile.suggestedFilename()).toContain('DRAFT-gstr1');
  expect(readFileSync((await csvFile.path())!, 'utf8')).toContain(
    '"OE","29-Karnataka","18.00","","1000.00"',
  );
  expect(
    (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21aa']).analyze())
      .violations,
  ).toEqual([]);
  await page.setViewportSize({ width: 320, height: 900 });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  );
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.getByLabel('GST return month / quarter end').fill('2026-10');
  await page.getByLabel('GST return frequency').selectOption('quarterly');
  await page.getByRole('button', { name: 'Prepare GST return draft', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('quarter-end');
  await expect(
    page.getByRole('heading', { name: 'GSTR-1 candidate sections', exact: true }),
  ).toHaveCount(0);
  const download = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Export GST register', exact: true }).click();
  expect((await download).suggestedFilename()).toContain('gst-register');
  await nav(page, 'Invoices & settlements');
  await page.getByRole('button', { name: 'Return / cancel', exact: true }).click();
  await page.getByLabel('Return reason').fill('GST full cancellation');
  await page.getByRole('checkbox', { name: /I reviewed GST adjustment eligibility/ }).check();
  await page.getByRole('button', { name: 'Cancel remaining invoice', exact: true }).click();
  await page.getByRole('button', { name: 'Review return', exact: true }).click();
  await expect(page.getByLabel('Invoice return')).toContainText('₹1,180.00');
  await page.getByRole('button', { name: 'Post return note', exact: true }).click();
  await expect(page.getByRole('button', { name: 'CN-000001', exact: true })).toBeVisible();
  const report = await api(request, 'gst_report', { from: '2026-01-01', to: '2026-12-31' });
  expect(report.totals.sale.taxable_minor).toBe('0');
  expect(report.totals.sale.cgst_minor).toBe('0');
  expect(report.rows[1].original_invoice).toBe('INV-000001');
});

test('experimental Tally XML mapping, preview and import work through the interface', async ({
  page,
  request,
}) => {
  await company(page, request, 'Tally mapping UI');
  await nav(page, 'Company & periods');
  await page
    .getByLabel('Tally voucher XML')
    .fill(readFileSync('crates/ledger-store/tests/fixtures/tally-general.xml', 'utf8'));
  await page.getByRole('button', { name: 'Inspect Tally XML', exact: true }).click();
  await page.getByLabel('Map Tally ledger: Bank & Cash').selectOption('2');
  await page.getByLabel('Map Tally ledger: Capital').selectOption('6');
  await page.getByLabel('Stable source company ID').fill('synthetic-book');
  await page
    .getByRole('checkbox', { name: 'I confirmed the source and destination books use INR.' })
    .check();
  await page.getByRole('button', { name: 'Preview mapped Tally import', exact: true }).click();
  await expect(page.getByText(/1 vouchers validated/)).toBeVisible();
  expect((await api(request, 'state')).journal_count).toBe(0);
  await page.getByRole('button', { name: 'Import mapped Tally vouchers', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('Tally general vouchers imported');
  expect((await api(request, 'state')).journal_count).toBe(1);
});

test('reviewed nil-rated, exempt and non-GST bills export separately and retain linked returns', async ({
  page,
  request,
}) => {
  await company(page, request, 'Classified supplies UI');
  const accounts = await api(request, 'gst_control_accounts');
  await api(request, 'configure_gst', {
    gstin: '29ABCDE1234F1Z5',
    state_code: '29',
    state_name: 'Karnataka',
    address: 'Synthetic registered office',
    ...accounts,
  });
  await api(request, 'create_party', {
    name: 'Classified supply buyer',
    kind: 'customer',
    contact: '',
    account_id: 3,
  });
  await page.reload();
  await nav(page, 'Invoices & settlements');
  for (const classification of ['nil_rated', 'exempt', 'non_gst']) {
    await page.getByRole('button', { name: 'New sales invoice', exact: true }).click();
    await page
      .getByLabel('Customer', { exact: true })
      .selectOption({ label: 'Classified supply buyer' });
    await page
      .getByLabel('Description', { exact: true })
      .fill(`Reviewed ${classification} service`);
    await page.getByLabel('Unit price', { exact: true }).fill('1000');
    await page.getByLabel('Posting account').selectOption('7');
    await page.getByRole('checkbox', { name: 'Record India tax details', exact: true }).check();
    await page.getByLabel('Supply classification', { exact: true }).selectOption(classification);
    await page
      .getByLabel('Reviewed classification basis')
      .fill('Synthetic reviewed legal reference');
    await page.getByLabel('Counterparty postal address').fill('Synthetic recipient office');
    await page.getByLabel('Delivery address').fill('Synthetic service address');
    await page.getByLabel('HSN / SAC', { exact: true }).fill('998314');
    await expect(page.getByLabel('Reviewed GST rate (%)', { exact: true })).toBeDisabled();
    await page.getByRole('button', { name: 'Review document', exact: true }).click();
    await expect(page.getByText('Total: ₹1,000.00', { exact: true })).toBeVisible();
    await expect(
      page.getByText('Synthetic reviewed legal reference', { exact: false }),
    ).toBeVisible();
    await page.getByRole('button', { name: 'Post sales invoice', exact: true }).click();
    await expect(
      page.getByRole('button', { name: 'New sales invoice', exact: true }),
    ).toBeVisible();
  }
  await page.getByRole('button', { name: 'INV-000001', exact: true }).click();
  await expect(page.getByText('Nil-rated supply', { exact: true })).toBeVisible();
  const downloading = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Export document (HTML)', exact: true }).click();
  const exported = await downloading;
  const html = readFileSync((await exported.path())!, 'utf8');
  expect(html).toContain('Bill of supply');
  expect(html).toContain('No GST charged.');
  expect(html).toContain('Synthetic reviewed legal reference');
  expect(html).not.toContain('<th>Rate %</th>');
  const invoiceRow = page
    .getByRole('row')
    .filter({ has: page.getByRole('button', { name: 'INV-000001', exact: true }) });
  await invoiceRow.getByRole('button', { name: 'Return / cancel', exact: true }).click();
  await page.getByLabel('Return reason', { exact: true }).fill('Partial nil-rated return');
  await page.getByLabel(/Return quantity 1:/).fill('0.5');
  await expect(
    page.getByRole('checkbox', { name: /I reviewed GST adjustment eligibility/ }),
  ).toHaveCount(0);
  await page.getByRole('button', { name: 'Review return', exact: true }).click();
  await page.getByRole('button', { name: 'Post return note', exact: true }).click();
  await expect(page.getByRole('button', { name: 'CN-000001', exact: true })).toBeVisible();
  await api(request, 'create_party', {
    name: 'Exempt supplier',
    kind: 'supplier',
    contact: '',
    account_id: 5,
  });
  await page.reload();
  await nav(page, 'Invoices & settlements');
  await page.getByRole('button', { name: 'New purchase invoice', exact: true }).click();
  await page.getByLabel('Supplier', { exact: true }).selectOption({ label: 'Exempt supplier' });
  await page.getByLabel('Description', { exact: true }).fill('Reviewed exempt purchase');
  await page.getByLabel('Unit price', { exact: true }).fill('1000');
  await page.getByLabel('Posting account').selectOption('8');
  await page.getByRole('checkbox', { name: 'Record India tax details', exact: true }).check();
  await page.getByLabel('Supply classification', { exact: true }).selectOption('exempt');
  await page.getByLabel('Reviewed supplier state code').fill('27');
  await page
    .getByLabel('Reviewed classification basis')
    .fill('Synthetic reviewed purchase exemption');
  await page.getByLabel('Supplier document reference').fill('SUP-NIL-1');
  await page.getByLabel('Counterparty postal address').fill('Synthetic Maharashtra supplier');
  await page.getByLabel('Delivery address').fill('Synthetic Karnataka recipient');
  await page.getByLabel('HSN / SAC', { exact: true }).fill('998314');
  await expect(
    page.getByRole('checkbox', { name: 'I independently reviewed input tax credit eligibility.' }),
  ).toHaveCount(0);
  await page.getByRole('button', { name: 'Review document', exact: true }).click();
  await page.getByRole('button', { name: 'Post purchase invoice', exact: true }).click();
  await expect(page.getByRole('button', { name: 'BILL-000001', exact: true })).toBeVisible();
  await nav(page, 'Financial statements');
  await page.getByRole('button', { name: 'Prepare GST return draft', exact: true }).click();
  const downloadingDraft = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Export complete GST draft (JSON)', exact: true }).click();
  const file = await downloadingDraft;
  const draft = JSON.parse(readFileSync((await file.path())!, 'utf8'));
  expect(draft.gstr3b_book_3_1_a.taxable_minor).toBe('0');
  expect(draft.gstr3b_book_3_1_c.taxable_minor).toBe('150000');
  expect(draft.gstr3b_book_3_1_e.taxable_minor).toBe('100000');
  expect(draft.gstr1_sections.exemp.taxable_minor).toBe('250000');
  expect(draft.gstr3b_book_5[0].inter_state_minor).toBe('100000');
  expect(draft.gstr3b_book_4_a_5.igst_minor).toBe('0');
  expect(draft.issues).toEqual([]);
  expect(
    (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21aa']).analyze())
      .violations,
  ).toEqual([]);
});
