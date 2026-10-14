export type Kind = 'asset' | 'liability' | 'equity' | 'income' | 'expense';
export type Side = 'debit' | 'credit';
export interface Company {
  name: string;
  currency: string;
  fiscal_year_start: string;
  created_at: string;
}
export interface Account {
  id: number;
  code: string;
  name: string;
  kind: Kind;
}
export interface Line {
  account_id: number;
  account_code: string;
  account_name: string;
  side: Side;
  amount_minor: string;
}
export interface Journal {
  id: number;
  date: string;
  reference: string;
  description: string;
  created_at: string;
  total_minor: string;
  reversal_of: number | null;
  reversed_by: number | null;
  lines: Line[];
}
export interface BalanceRow {
  account: Account;
  net_debit_minor: string;
  debit_minor: string;
  credit_minor: string;
}
export interface TrialBalance {
  rows: BalanceRow[];
  debit_minor: string;
  credit_minor: string;
  balanced: boolean;
  as_of: string | null;
}
export interface Snapshot {
  company: Company | null;
  accounts: Account[];
  journals: Journal[];
  journal_count: number;
  journal_activity: Record<string, number>;
  trial_balance: TrialBalance;
  statements: { income_minor: string; expense_minor: string; profit_minor: string };
}
export interface LedgerLine {
  journal_id: number;
  date: string;
  reference: string;
  description: string;
  debit_minor: string;
  credit_minor: string;
  balance_minor: string;
}
export interface Ledger {
  line_count?: number;
  account: Account;
  lines: LedgerLine[];
  opening_minor: string;
  closing_minor: string;
  debit_minor: string;
  credit_minor: string;
}
export interface BackupPreview {
  company_name: string;
  currency: string;
  account_count: number;
  journal_count: number;
  exported_at: string;
}
declare global {
  interface Window {
    __TAURI__?: { core: { invoke: <T>(name: string, args?: object) => Promise<T> } };
  }
}
export const desktop = () => Boolean(window.__TAURI__?.core);
export async function request<T>(command: string, payload: object = {}): Promise<T> {
  if (desktop()) return window.__TAURI__!.core.invoke<T>('ledger_request', { command, payload });
  const response = await fetch(`/api/${command}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  });
  const value = await response.json();
  if (!response.ok) throw new Error(value.error || 'The request could not be completed.');
  return value as T;
}
export const today = () => {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
};
export function parseMinor(text: string): bigint | null {
  const value = text.trim();
  if (!/^(?:\d+(?:\.\d{0,2})?|\.\d{1,2})$/.test(value)) return null;
  const [whole, fraction = ''] = value.split('.');
  return BigInt(whole || '0') * 100n + BigInt(fraction.padEnd(2, '0') || '0');
}
export function decimal(value: bigint): string {
  return `${value / 100n}.${String(value % 100n).padStart(2, '0')}`;
}
export function displayLocale(): string {
  let preference: string | null = null;
  try {
    preference = localStorage.getItem('open-ledger-locale');
  } catch {
    // Optional display preferences must not prevent access to the books.
  }
  for (const candidate of [preference, navigator.language, 'en-IN']) {
    if (!candidate) continue;
    try {
      const [locale] = Intl.getCanonicalLocales(candidate);
      if (locale && Intl.NumberFormat.supportedLocalesOf([locale]).length) return locale;
    } catch {
      // Some native environments report POSIX values such as C or C.UTF-8.
    }
  }
  return 'en-IN';
}
export function money(value: string | bigint, currency: string, symbol = true): string {
  const amount = BigInt(value);
  const absolute = amount < 0n ? -amount : amount;
  const locale = displayLocale();
  const parts = new Intl.NumberFormat(locale, {
    style: 'currency',
    currency,
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).formatToParts(absolute / 100n);
  const fraction = String(absolute % 100n).padStart(2, '0');
  const rendered = parts
    .filter((p) => symbol || (p.type !== 'currency' && p.type !== 'literal'))
    .map((p) => (p.type === 'fraction' ? fraction : p.value))
    .join('');
  return `${amount < 0n ? '−' : ''}${rendered}`;
}
export function displayDate(value: string): string {
  return new Date(`${value}T12:00:00`).toLocaleDateString(displayLocale(), {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
  });
}
export async function download(name: string, contents: string, type = 'application/json') {
  if (desktop()) {
    try {
      await window.__TAURI__!.core.invoke('save_export', { filename: name, contents });
    } catch (e) {
      window.dispatchEvent(new CustomEvent('open-ledger-error', { detail: String(e) }));
    }
    return;
  }
  const url = URL.createObjectURL(new Blob([contents], { type }));
  const a = document.createElement('a');
  a.href = url;
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
export function csv(rows: (string | number)[][]) {
  return rows
    .map((row) =>
      row
        .map((cell) => {
          let value = String(cell);
          if (
            typeof cell === 'string' &&
            /^[=+@\-]/.test(value) &&
            !/^-?\d+(?:\.\d+)?$/.test(value)
          )
            value = `'${value}`;
          return `"${value.replaceAll('"', '""')}"`;
        })
        .join(','),
    )
    .join('\r\n');
}
