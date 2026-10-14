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
