import { BusinessWorkspace, BusinessPage } from './business';
import React, { createContext, useContext, useEffect, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import {
  Account,
  BackupPreview,
  BalanceRow,
  Journal,
  Kind,
  Ledger,
  Side,
  Snapshot,
  TrialBalance,
  csv,
  decimal,
  desktop,
  displayDate,
  download,
  money,
  parseMinor,
  request,
  today,
} from './api';
import './styles.css';

type Page = BusinessPage | 'overview' | 'journals' | 'accounts' | 'ledger' | 'trial' | 'backups';
type Modal = 'journal' | 'account' | { reverse: Journal } | null;
const kinds: Kind[] = ['asset', 'liability', 'equity', 'income', 'expense'];
const ErrorContext = createContext('');
const kindName: Record<Kind, string> = {
  asset: 'Asset',
  liability: 'Liability',
  equity: 'Equity',
  income: 'Income',
  expense: 'Expense',
};
const paths: Record<string, React.ReactNode> = {
  overview: (
    <>
      <rect x="3" y="3" width="7" height="7" rx="1" />
      <rect x="14" y="3" width="7" height="7" rx="1" />
      <rect x="3" y="14" width="7" height="7" rx="1" />
      <rect x="14" y="14" width="7" height="7" rx="1" />
    </>
  ),
  journals: (
    <>
      <path d="M5 3h14v18H5zM9 7h6M9 11h6M9 15h4" />
    </>
  ),
  accounts: (
    <>
      <path d="M4 8h16M6 8v11m6-11v11m6-11v11M3 21h18M3 6l9-4 9 4" />
    </>
  ),
  ledger: (
    <>
      <path d="M3 4h7l2 2 2-2h7v16h-7l-2 2-2-2H3zM12 6v16" />
    </>
  ),
  trial: (
    <>
      <path d="M12 3v17M8 21h8M5 6h14M5 6l-3 7h6L5 6Zm14 0-3 7h6l-3-7Z" />
    </>
  ),
  backups: (
    <>
      <path d="M5 4h14v16H5zM9 4v6h6V4M9 20v-6h6v6" />
    </>
  ),
  plus: <path d="M12 5v14M5 12h14" />,
  arrow: <path d="m9 5 7 7-7 7" />,
  check: <path d="m5 12 4 4L19 6" />,
  close: <path d="m6 6 12 12M6 18 18 6" />,
  download: <path d="M12 3v12m-5-5 5 5 5-5M4 17v4h16v-4" />,
  search: (
    <>
      <circle cx="10" cy="10" r="6" />
      <path d="m15 15 6 6" />
    </>
  ),
  reverse: <path d="m7 3-5 5 5 5M2 8h12a7 7 0 0 1 0 14" />,
  shield: (
    <>
      <path d="m12 2 8 3v7c0 5-8 10-8 10S4 17 4 12V5l8-3Z" />
      <path d="m8 12 3 3 5-6" />
    </>
  ),
  file: (
    <>
      <path d="M5 2h9l5 5v15H5zM14 2v6h5M9 13h6M9 17h6" />
    </>
  ),
  menu: <path d="M4 6h16M4 12h16M4 18h16" />,
  dot: <circle cx="12" cy="12" r="3" />,
};
function Icon({ name, size = 20 }: { name: string; size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {paths[name] || paths.file}
    </svg>
  );
}
function Brand() {
  return (
    <div className="brand">
      <span className="brand-mark">
        <i />
        <i />
        <i />
        <i />
      </span>
      <span>
        open<span className="brand-light">ledger</span>
      </span>
    </div>
  );
}
function Badge({ children, neutral = false }: { children: React.ReactNode; neutral?: boolean }) {
  return <span className={`badge ${neutral ? 'neutral' : ''}`}>{children}</span>;
}
function Empty({
  title,
  children,
  action,
}: {
  title: string;
  children: React.ReactNode;
  action?: React.ReactNode;
}) {
  return (
    <div className="empty">
      <span className="empty-icon">
        <Icon name="ledger" size={28} />
      </span>
      <h3>{title}</h3>
      <p>{children}</p>
      {action}
    </div>
  );
}
function ModalFrame({
  title,
  eyebrow,
  close,
  children,
  wide = false,
}: {
  title: string;
  eyebrow: string;
  close: () => void;
  children: React.ReactNode;
  wide?: boolean;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const closeRef = useRef(close);
  closeRef.current = close;
  const error = useContext(ErrorContext);
  useEffect(() => {
    const prior = document.activeElement as HTMLElement;
    (
      ref.current?.querySelector<HTMLElement>('input,select') ||
      ref.current?.querySelector<HTMLElement>('button')
    )?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') closeRef.current();
      if (e.key === 'Tab') {
        const items = Array.from(
          ref.current?.querySelectorAll<HTMLElement>(
            'button:not(:disabled),input:not(:disabled),select:not(:disabled),textarea:not(:disabled)',
          ) || [],
        );
        if (!items.length) return;
        if (e.shiftKey && document.activeElement === items[0]) {
          e.preventDefault();
          items.at(-1)?.focus();
        } else if (!e.shiftKey && document.activeElement === items.at(-1)) {
          e.preventDefault();
          items[0]?.focus();
        }
      }
    };
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('keydown', onKey);
      prior?.focus();
    };
  }, []);
  return (
    <div className="modal-backdrop">
      <div
        className={`modal ${wide ? 'wide' : ''}`}
        role="dialog"
        aria-modal="true"
        aria-labelledby="modal-title"
        ref={ref}
      >
        <div className="modal-heading">
          <div>
            <p className="eyebrow">{eyebrow}</p>
            <h2 id="modal-title">{title}</h2>
          </div>
          <button className="icon-button" onClick={close} aria-label="Close dialog">
            <Icon name="close" />
          </button>
        </div>
        {error && (
          <div className="form-error" role="alert">
            {error}
          </div>
        )}
        {children}
      </div>
    </div>
  );
}
function App() {
  const [state, setState] = useState<Snapshot | null>(null);
  const [page, setPage] = useState<Page>('overview');
  const [modal, setModal] = useState<Modal>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [toast, setToast] = useState('');
  const [selectedJournal, setSelectedJournal] = useState<Journal | null>(null);
  const [selectedAccount, setSelectedAccount] = useState(0);
  const [sidebar, setSidebar] = useState(false);
  const reload = async () => {
    try {
      setState(await request<Snapshot>('state'));
      setError('');
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    }
  };
  useEffect(() => {
    void reload();
    const onExportError = (event: Event) => setError(String((event as CustomEvent).detail));
    window.addEventListener('open-ledger-error', onExportError);
    return () => window.removeEventListener('open-ledger-error', onExportError);
  }, []);
  useEffect(() => {
    if (!toast) return;
    const t = setTimeout(() => setToast(''), 4500);
    return () => clearTimeout(t);
  }, [toast]);
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'j' && state?.company) {
        e.preventDefault();
        setModal('journal');
      }
    };
    window.addEventListener('keydown', key);
    return () => window.removeEventListener('keydown', key);
  }, [state?.company]);
  const mutate = async (command: string, payload: object, message: string): Promise<boolean> => {
    setBusy(true);
    setError('');
    try {
      await request(command, payload);
      await reload();
      if (['new_books', 'switch_books'].includes(command)) {
        setSelectedAccount(0);
        setSelectedJournal(null);
        setModal(null);
        setPage('overview');
      }
      setToast(message);
      return true;
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
      return false;
    } finally {
      setBusy(false);
    }
  };
  const exportBackup = async () => {
    setBusy(true);
    try {
      if (desktop()) {
        const r = await window.__TAURI__!.core.invoke<{ saved: boolean }>('save_backup');
        if (r.saved) setToast('Backup saved.');
      } else {
        const text = await request<string>('backup');
        download(`open-ledger-backup-${today()}.json`, text);
        setToast('Backup downloaded.');
      }
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    } finally {
      setBusy(false);
    }
  };
  const navigate = (next: Page) => {
    setError('');
    setPage(next);
    setSidebar(false);
    setSelectedJournal(null);
  };
  if (!state)
    return (
      <div className="loading-screen">
        <Brand />
        <p>{error || 'Opening your books…'}</p>
        {error && (
          <button className="button primary" onClick={() => void reload()}>
            Try again
          </button>
        )}
      </div>
    );
  if (!state.company) return <Welcome busy={busy} error={error} mutate={mutate} />;
  const currency = state.company.currency;
  const titles: Record<Page, [string, string]> = {
    overview: ['Overview', 'A clear view of your company’s books.'],
    journals: ['Journal entries', 'Every posting, with its complete accounting trail.'],
    accounts: ['Chart of accounts', 'The structure behind your financial records.'],
    ledger: ['Account ledger', 'Follow the movement in an individual account.'],
    trial: ['Trial balance', 'Closing balances across every account.'],
    documents: ['Invoices & settlements', 'Customers, suppliers, and outstanding invoices.'],
    inventory: ['Inventory', 'Stock movements and carrying values.'],
    statements: ['Financial statements', 'Profit and loss and balance sheet.'],
    reconciliation: ['Bank reconciliation', 'Match your books to bank statements.'],
    settings: ['Company & periods', 'Company switching, numbering, closing, and import.'],
    backups: ['Backups & restore', 'Keep a recoverable copy of your books.'],
  };
  return (
    <ErrorContext.Provider value={error}>
      <div className="app-shell">
        <aside className={`sidebar ${sidebar ? 'open' : ''}`}>
          <Brand />
          <div className="workspace">
            <span className="company-avatar">{state.company.name[0]?.toUpperCase()}</span>
            <div>
              <strong>{state.company.name}</strong>
              <small>{currency} · Company books</small>
            </div>
          </div>
          <p className="nav-label">WORKSPACE</p>
          <nav>
            {(
              [
                'overview',
                'documents',
                'inventory',
                'journals',
                'accounts',
                'ledger',
                'trial',
                'statements',
                'reconciliation',
                'settings',
                'backups',
              ] as Page[]
            ).map((item) => (
              <button
                key={item}
                className={page === item ? 'active' : ''}
                onClick={() => navigate(item)}
              >
                <Icon name={item} />
                <span>
                  {
                    {
                      overview: 'Overview',
                      journals: 'Journal entries',
                      accounts: 'Chart of accounts',
                      ledger: 'Account ledger',
                      trial: 'Trial balance',
                      documents: 'Invoices & settlements',
                      inventory: 'Inventory',
                      statements: 'Financial statements',
                      reconciliation: 'Bank reconciliation',
                      settings: 'Company & periods',
                      backups: 'Backups',
                    }[item]
                  }
                </span>
                {item === 'journals' && <small>{state.journal_count}</small>}
              </button>
            ))}
          </nav>
          <div className="sidebar-bottom">
            <div className="local-status">
              <span />
              Stored on this device
            </div>
            <p>Your records stay with you.</p>
            <span className="version">OPEN LEDGER · 0.1</span>
          </div>
        </aside>
        <main className="main">
          <header className="topbar">
            <div>
              <button
                className="icon-button mobile-menu"
                aria-label="Toggle navigation"
                onClick={() => setSidebar(!sidebar)}
              >
                <Icon name="menu" />
              </button>
              <span className="breadcrumb">Workspace</span>
              <Icon name="arrow" size={13} />
              <strong>{titles[page][0]}</strong>
            </div>
            <div className="topbar-right">
              <span className="today">{displayDate(today())}</span>
              <span className="profile">{state.company.name[0]?.toUpperCase()}</span>
            </div>
          </header>
          <div className="content">
            <div className="page-heading">
              <div>
                <p className="eyebrow">YOUR COMPANY BOOKS</p>
                <h1>{titles[page][0]}</h1>
                <p>{titles[page][1]}</p>
              </div>
              <div className="heading-actions">
                {page === 'accounts' ? (
                  <button className="button primary" onClick={() => setModal('account')}>
                    <Icon name="plus" size={17} />
                    Add account
                  </button>
                ) : (
                  ['overview', 'journals', 'ledger', 'trial'].includes(page) && (
                    <button className="button primary" onClick={() => setModal('journal')}>
                      <Icon name="plus" size={17} />
                      New journal<span className="shortcut">⌘ J</span>
                    </button>
                  )
                )}
              </div>
            </div>
            {error && (
              <div className="error-banner" role="alert">
                {error}
                <button
                  className="icon-button"
                  aria-label="Dismiss error"
                  onClick={() => setError('')}
                >
                  <Icon name="close" size={16} />
                </button>
              </div>
            )}
            {(
              ['documents', 'inventory', 'statements', 'reconciliation', 'settings'] as Page[]
            ).includes(page) && (
              <BusinessWorkspace
                key={`${page}-${state.company.created_at}`}
                page={page as BusinessPage}
                state={state}
                busy={busy}
                mutate={mutate}
                onError={setError}
                openJournal={setSelectedJournal}
              />
            )}
            {page === 'overview' && (
              <Overview
                state={state}
                openJournal={setSelectedJournal}
                newJournal={() => setModal('journal')}
                go={navigate}
              />
            )}
            {page === 'journals' && (
              <Journals
                state={state}
                open={setSelectedJournal}
                newJournal={() => setModal('journal')}
              />
            )}
            {page === 'accounts' && (
              <Accounts
                state={state}
                open={(id) => {
                  setSelectedAccount(id);
                  navigate('ledger');
                }}
                newAccount={() => setModal('account')}
              />
            )}
            {page === 'ledger' && (
              <LedgerView
                state={state}
                selected={selectedAccount}
                select={setSelectedAccount}
                openJournal={(id) => {
                  const found = state.journals.find((j) => j.id === id);
                  if (found) setSelectedJournal(found);
                  else
                    request<Journal>('journal', { id })
                      .then(setSelectedJournal)
                      .catch((e) => setError(String(e.message || e)));
                }}
                onError={setError}
              />
            )}
            {page === 'trial' && <TrialView state={state} onError={setError} />}
            {page === 'backups' && (
              <Backups
                state={state}
                busy={busy}
                exportBackup={exportBackup}
                mutate={mutate}
                onError={setError}
              />
            )}
            <footer className="page-footer">
              <span>
                <Icon name="shield" size={14} />
                Exact amounts. Balanced postings. Recoverable books.
              </span>
              <span>
                {currency} · Financial year begins {displayDate(state.company.fiscal_year_start)}
              </span>
            </footer>
          </div>
        </main>
        {toast && (
          <div className="toast" role="status">
            <Icon name="check" size={18} />
            {toast}
          </div>
        )}
        {modal === 'journal' && (
          <JournalComposer
            state={state}
            busy={busy}
            close={() => {
              if (!busy) setModal(null);
            }}
            mutate={mutate}
          />
        )}
        {modal === 'account' && (
          <AccountComposer
            busy={busy}
            close={() => {
              if (!busy) setModal(null);
            }}
            mutate={mutate}
          />
        )}
        {typeof modal === 'object' && modal?.reverse && (
          <ReverseComposer
            journal={modal.reverse}
            busy={busy}
            close={() => {
              if (!busy) setModal(null);
            }}
            mutate={mutate}
          />
        )}
        {selectedJournal && (
          <JournalDetails
            journal={selectedJournal}
            currency={currency}
            close={() => setSelectedJournal(null)}
            reverse={() => {
              setModal({ reverse: selectedJournal });
              setSelectedJournal(null);
            }}
          />
        )}
      </div>
    </ErrorContext.Provider>
  );
}
type Mutation = (command: string, payload: object, message: string) => Promise<boolean>;
function Welcome({ busy, error, mutate }: { busy: boolean; error: string; mutate: Mutation }) {
  const [name, setName] = useState('');
  const [currency, setCurrency] = useState('INR');
  const [start, setStart] = useState(`${today().slice(0, 4)}-01-01`);
  const [books, setBooks] = useState<{ file: string; company: Snapshot['company'] }[]>([]);
  useEffect(() => {
    void request<{ books: typeof books }>('books')
      .then((v) => setBooks(v.books))
      .catch(() => {});
  }, []);
  return (
    <div className="welcome">
      <div className="welcome-story">
        <Brand />
        <div className="welcome-copy">
          <span className="intro-pill">YOUR BOOKS. YOUR DEVICE.</span>
          <h1>
            A little clarity.
            <br />A lot of balance.
          </h1>
          <p>
            A focused workspace for keeping company accounts, following every posting, and knowing
            where your numbers stand.
          </p>
          <div className="welcome-features">
            <span>
              <Icon name="check" />
              Balanced journal entries
            </span>
            <span>
              <Icon name="check" />
              Ledgers you can trace
            </span>
            <span>
              <Icon name="check" />
              Backups you can restore
            </span>
          </div>
        </div>
        <small>OPEN LEDGER · BUILT FOR EVERYDAY BOOKKEEPING</small>
        <div className="welcome-art">
          <div className="art-card">
            <span>TRIAL BALANCE</span>
            <div>
              <i />
              <i />
              <i />
            </div>
            <hr />
            <strong>
              In balance <Icon name="check" size={16} />
            </strong>
          </div>
          <span className="art-orbit" />
        </div>
      </div>
      <div className="welcome-form">
        <div className="setup-card">
          <p className="eyebrow">LET’S GET STARTED</p>
          <h2>Create your company</h2>
          <p>Start with a simple chart of accounts. Add the accounts you need as you go.</p>
          {error && (
            <div className="form-error" role="alert">
              {error}
            </div>
          )}
          <form
            onSubmit={async (e) => {
              e.preventDefault();
              await mutate(
                'create_company',
                { name, currency, fiscal_year_start: start },
                'Your company is ready.',
              );
            }}
          >
            <label>
              Company name
              <input
                required
                maxLength={120}
                autoFocus
                placeholder="e.g. Northstar Studio"
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
            </label>
            <div className="form-columns">
              <label>
                Base currency
                <select value={currency} onChange={(e) => setCurrency(e.target.value)}>
                  {['INR', 'USD', 'EUR', 'GBP', 'CAD', 'AUD'].map((c) => (
                    <option key={c}>{c}</option>
                  ))}
                </select>
              </label>
              <label>
                Financial year begins
                <input
                  required
                  type="date"
                  value={start}
                  onChange={(e) => setStart(e.target.value)}
                />
              </label>
            </div>
            <button className="button primary full" disabled={busy}>
              {busy ? 'Creating your books…' : 'Create company'}
              <Icon name="arrow" size={17} />
            </button>
          </form>
          <div className="setup-divider">
            <span>OR EXPLORE FIRST</span>
          </div>
          <button
            className="button secondary full"
            disabled={busy}
            onClick={async () => {
              if (await mutate('new_books', {}, 'Example file created.'))
                await mutate('create_demo', {}, 'Example books are ready.');
            }}
          >
            Try example books
          </button>
          {books.some((b) => b.company) && (
            <label>
              Open existing company
              <select
                value=""
                disabled={busy}
                onChange={(e) =>
                  void mutate('switch_books', { file: e.target.value }, 'Company opened.')
                }
              >
                <option value="">Select company</option>
                {books
                  .filter((b) => b.company)
                  .map((b) => (
                    <option key={b.file} value={b.file}>
                      {b.company!.name}
                    </option>
                  ))}
              </select>
            </label>
          )}
          <p className="setup-note">
            <Icon name="shield" size={16} />
            Books are saved locally. No account or internet connection is required in the desktop
            app.
          </p>
        </div>
      </div>
    </div>
  );
}
function Overview({
  state,
  openJournal,
  newJournal,
  go,
}: {
  state: Snapshot;
  openJournal: (j: Journal) => void;
  newJournal: () => void;
  go: (p: Page) => void;
}) {
  const currency = state.company!.currency;
  const values = Object.fromEntries(
    kinds.map((kind) => [
      kind,
      state.trial_balance.rows
        .filter((row) => row.account.kind === kind)
        .reduce((sum, row) => sum + BigInt(row.net_debit_minor), 0n),
    ]),
  ) as Record<Kind, bigint>;
  const income = BigInt(state.statements.income_minor);
  const expenses = BigInt(state.statements.expense_minor);
  const profit = income - expenses;
  const metrics: [string, bigint, string, string][] = [
    ['Total assets', values.asset, 'accounts', 'Cash, receivables & other assets'],
    ['Total liabilities', -values.liability, 'ledger', 'Amounts your company owes'],
    ['Income', income, 'journals', 'Posted income across your books'],
    ['Expenses', expenses, 'file', 'Posted operating costs'],
  ];
  const bars = Array.from({ length: 14 }, (_, i) => {
    const d = new Date(`${today()}T12:00:00`);
    d.setDate(d.getDate() - 13 + i);
    const date = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
    return { date, count: state.journal_activity[date] || 0 };
  });
  const max = Math.max(1, ...bars.map((b) => b.count));
  return (
    <>
      <div className="overview-banner">
        <div>
          <span className="banner-icon">
            <Icon name="trial" size={25} />
          </span>
          <div>
            <strong>
              Your trial balance is {state.trial_balance.balanced ? 'in balance' : 'out of balance'}
              .
            </strong>
            <p>
              {state.journal_count
                ? `${state.journal_count} posted ${state.journal_count === 1 ? 'entry' : 'entries'} · Every debit accounted for.`
                : 'Your chart is ready. Add your first balanced entry.'}
            </p>
          </div>
        </div>
        <button onClick={() => go('trial')}>
          View trial balance
          <Icon name="arrow" size={15} />
        </button>
      </div>
      <div className="metrics">
        {metrics.map(([label, amount, icon, note]) => (
          <div className="metric" key={label}>
            <div>
              <span>{label}</span>
              <Icon name={icon} size={19} />
            </div>
            <strong>{money(amount, currency)}</strong>
            <small>{note}</small>
          </div>
        ))}
      </div>
      <div className="overview-middle">
        <section className="panel activity-panel">
          <div className="panel-title">
            <div>
              <h3>Journal activity</h3>
              <p>Number of entries · last 14 days</p>
            </div>
            <Badge neutral>{state.journal_count} entries</Badge>
          </div>
          <div className="activity-chart">
            {bars.map((bar) => (
              <div
                className="bar-column"
                key={bar.date}
                title={`${displayDate(bar.date)}: ${bar.count} entries`}
              >
                <div
                  className={`chart-bar ${bar.count ? '' : 'zero'}`}
                  style={{ height: `${Math.max(3, (bar.count / max) * 100)}%` }}
                />
              </div>
            ))}
          </div>
          <div className="chart-labels">
            <span>{displayDate(bars[0].date)}</span>
            <span>{displayDate(today())}</span>
          </div>
        </section>
        <section className="panel profit-panel">
          <div className="panel-title">
            <div>
              <h3>Income less expenses</h3>
              <p>Across all posted entries</p>
            </div>
            <Icon name="overview" size={20} />
          </div>
          <strong className="profit-number">{money(profit, currency)}</strong>
          <Badge>{profit >= 0n ? 'Net surplus' : 'Net deficit'}</Badge>
          <div className="profit-breakdown">
            <div>
              <span>Income</span>
              <strong>{money(income, currency)}</strong>
            </div>
            <div>
              <span>Expenses</span>
              <strong>{money(expenses, currency)}</strong>
            </div>
          </div>
        </section>
      </div>
      <section className="panel">
        <div className="panel-title">
          <div>
            <h3>Recent journal entries</h3>
            <p>The latest activity in your books</p>
          </div>
          <button className="text-button" onClick={() => go('journals')}>
            View all entries
            <Icon name="arrow" size={15} />
          </button>
        </div>
        {state.journals.length ? (
          <JournalTable
            journals={state.journals.slice(0, 5)}
            currency={currency}
            open={openJournal}
          />
        ) : (
          <Empty
            title="A fresh start"
            action={
              <button className="button primary" onClick={newJournal}>
                <Icon name="plus" size={16} />
                Post your first entry
              </button>
            }
          >
            Record your opening capital or your first company transaction.
          </Empty>
        )}
      </section>
    </>
  );
}
function JournalTable({
  journals,
  currency,
  open,
}: {
  journals: Journal[];
  currency: string;
  open: (j: Journal) => void;
}) {
  return (
    <div className="table-scroll">
      <table>
        <thead>
          <tr>
            <th>Date</th>
            <th>Reference & description</th>
            <th>Status</th>
            <th className="numeric">Journal amount</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {journals.map((j) => (
            <tr key={j.id}>
              <td className="date-cell">{displayDate(j.date)}</td>
              <td>
                <button className="table-link" onClick={() => open(j)}>
                  {j.reference}
                </button>
                <small className="cell-note">{j.description}</small>
              </td>
              <td>
                <Badge neutral={Boolean(j.reversed_by || j.reversal_of)}>
                  {j.reversed_by ? 'Reversed' : j.reversal_of ? 'Reversal' : 'Posted'}
                </Badge>
              </td>
              <td className="numeric strong">{money(j.total_minor, currency)}</td>
              <td>
                <button
                  className="icon-button"
                  onClick={() => open(j)}
                  aria-label={`View ${j.reference}`}
                >
                  <Icon name="arrow" size={17} />
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
function Journals({
  state,
  open,
  newJournal,
}: {
  state: Snapshot;
  open: (j: Journal) => void;
  newJournal: () => void;
}) {
  const [search, setSearch] = useState('');
  const [offset, setOffset] = useState(0);
  const [journals, setJournals] = useState<Journal[]>(state.journals.slice(0, 50));
  const [loadError, setLoadError] = useState('');
  useEffect(() => {
    let live = true;
    void request<Journal[]>('journals_page', { offset, limit: 50 })
      .then((v) => {
        if (live) {
          setJournals(v);
          setLoadError('');
        }
      })
      .catch((e) => {
        if (live) setLoadError(String(e.message || e));
      });
    return () => {
      live = false;
    };
  }, [offset, state]);
  const records = journals.filter((j) =>
    `${j.reference} ${j.description} ${j.date}`.toLowerCase().includes(search.toLowerCase()),
  );
  return (
    <section className="panel">
      <div className="panel-toolbar">
        <div className="search-input">
          <Icon name="search" size={18} />
          <input
            aria-label="Search journal entries on this page"
            placeholder="Search this page…"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>
        <span className="muted">
          {records.length} shown · {state.journal_count} total
        </span>
      </div>
      {loadError && (
        <p role="alert" className="form-error">
          {loadError}
        </p>
      )}
      {records.length ? (
        <JournalTable journals={records} currency={state.company!.currency} open={open} />
      ) : (
        <Empty
          title={search ? 'No matching entries' : 'Your journal starts here'}
          action={
            !search && (
              <button className="button primary" onClick={newJournal}>
                New journal entry
              </button>
            )
          }
        >
          {search
            ? 'Try another reference or description.'
            : 'Post a balanced entry to begin building your books.'}
        </Empty>
      )}
      <div className="panel-toolbar">
        <button
          className="button secondary"
          disabled={!offset}
          onClick={() => setOffset(Math.max(0, offset - 50))}
        >
          Previous
        </button>
        <span>Page {offset / 50 + 1}</span>
        <button
          className="button secondary"
          disabled={offset + 50 >= state.journal_count}
          onClick={() => setOffset(offset + 50)}
        >
          Next
        </button>
      </div>
    </section>
  );
}
function Accounts({
  state,
  open,
  newAccount,
}: {
  state: Snapshot;
  open: (id: number) => void;
  newAccount: () => void;
}) {
  const [filter, setFilter] = useState('all');
  const rows = state.trial_balance.rows.filter(
    (r) => filter === 'all' || r.account.kind === filter,
  );
  return (
    <section className="panel">
      <div className="panel-toolbar">
        <div className="tabs">
          <button className={filter === 'all' ? 'selected' : ''} onClick={() => setFilter('all')}>
            All accounts
          </button>
          {kinds.map((k) => (
            <button className={filter === k ? 'selected' : ''} onClick={() => setFilter(k)} key={k}>
              {kindName[k]}
            </button>
          ))}
        </div>
        <span className="muted">{state.accounts.length} accounts</span>
      </div>
      <div className="table-scroll">
        <table>
          <thead>
            <tr>
              <th>Code</th>
              <th>Account name</th>
              <th>Type</th>
              <th className="numeric">Closing balance</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr key={row.account.id}>
                <td className="account-code">{row.account.code}</td>
                <td>
                  <button className="table-link" onClick={() => open(row.account.id)}>
                    {row.account.name}
                  </button>
                </td>
                <td>
                  <span className={`kind-dot ${row.account.kind}`} />
                  {kindName[row.account.kind]}
                </td>
                <td className="numeric">
                  {balanceText(row.net_debit_minor, state.company!.currency)}
                </td>
                <td>
                  <button
                    className="icon-button"
                    onClick={() => open(row.account.id)}
                    aria-label={`Open ${row.account.name} ledger`}
                  >
                    <Icon name="arrow" size={17} />
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {!rows.length && (
        <Empty
          title="No accounts in this category"
          action={
            <button className="button secondary" onClick={newAccount}>
              Add account
            </button>
          }
        >
          Create an account to organize this part of your books.
        </Empty>
      )}
      <div className="panel-footnote">
        Opening balances are recorded as journal entries, keeping the books balanced.
      </div>
    </section>
  );
}
function balanceText(value: string, currency: string) {
  const n = BigInt(value);
  return `${money(n < 0n ? -n : n, currency)}${n ? (n > 0n ? ' Dr' : ' Cr') : ''}`;
}
function LedgerView({
  state,
  selected,
  select,
  openJournal,
  onError,
}: {
  state: Snapshot;
  selected: number;
  select: (id: number) => void;
  openJournal: (id: number) => void;
  onError: (e: string) => void;
}) {
  const account =
    selected || state.accounts.find((a) => a.code === '1100')?.id || state.accounts[0]?.id;
  const [from, setFrom] = useState('');
  const [to, setTo] = useState('');
  const [data, setData] = useState<Ledger | null>(null);
  const [offset, setOffset] = useState(0);
  useEffect(() => setOffset(0), [account, from, to]);
  const [loading, setLoading] = useState(false);
  useEffect(() => {
    let active = true;
    setLoading(true);
    request<Ledger>('ledger_page', {
      account_id: account,
      from: from || null,
      to: to || null,
      offset,
      limit: 50,
    })
      .then((v) => {
        if (active) setData(v);
      })
      .catch((e) => {
        if (active) {
          setData(null);
          onError(String(e.message || e));
        }
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [account, from, to, state.journal_count, offset]);
  const currency = state.company!.currency;
  return (
    <>
      <section className="panel filter-panel">
        <label>
          Account
          <select value={account} onChange={(e) => select(Number(e.target.value))}>
            {state.accounts.map((a) => (
              <option key={a.id} value={a.id}>
                {a.code} · {a.name}
              </option>
            ))}
          </select>
        </label>
        <label>
          From date
          <input type="date" value={from} onChange={(e) => setFrom(e.target.value)} />
        </label>
        <label>
          Through date
          <input type="date" value={to} onChange={(e) => setTo(e.target.value)} />
        </label>
        <button
          className="button secondary"
          disabled={!data || loading}
          onClick={async () => {
            try {
              const all = await request<Ledger>('ledger', {
                account_id: account,
                from: from || null,
                to: to || null,
              });
              download(
                `ledger-${all.account.code}.csv`,
                csv([
                  [
                    'Date',
                    'Reference',
                    'Description',
                    `Debit (${currency})`,
                    `Credit (${currency})`,
                    'Running debit balance',
                  ],
                  ...all.lines.map((l) => [
                    l.date,
                    l.reference,
                    l.description,
                    money(l.debit_minor, currency, false),
                    money(l.credit_minor, currency, false),
                    money(l.balance_minor, currency, false),
                  ]),
                ]),
                'text/csv',
              );
            } catch (e) {
              onError(String(e instanceof Error ? e.message : e));
            }
          }}
        >
          <Icon name="download" size={16} />
          Export CSV
        </button>
      </section>
      {data && (
        <>
          <div className="ledger-summary">
            <div>
              <span>Opening balance</span>
              <strong>{balanceText(data.opening_minor, currency)}</strong>
            </div>
            <div>
              <span>Period debits</span>
              <strong>{money(data.debit_minor, currency)}</strong>
            </div>
            <div>
              <span>Period credits</span>
              <strong>{money(data.credit_minor, currency)}</strong>
            </div>
            <div>
              <span>Closing balance</span>
              <strong>{balanceText(data.closing_minor, currency)}</strong>
            </div>
          </div>
          <section className="panel">
            <div className="panel-title">
              <div>
                <h3>{data.account.name}</h3>
                <p>
                  {data.account.code} · {kindName[data.account.kind]}
                </p>
              </div>
              <Badge neutral>{data.line_count || 0} postings</Badge>
            </div>
            {data.lines.length ? (
              <div className="table-scroll">
                <table>
                  <thead>
                    <tr>
                      <th>Date</th>
                      <th>Reference & description</th>
                      <th className="numeric">Debit</th>
                      <th className="numeric">Credit</th>
                      <th className="numeric">Running balance</th>
                    </tr>
                  </thead>
                  <tbody>
                    {data.lines.map((l, i) => (
                      <tr key={i}>
                        <td className="date-cell">{displayDate(l.date)}</td>
                        <td>
                          <button className="table-link" onClick={() => openJournal(l.journal_id)}>
                            {l.reference}
                          </button>
                          <small className="cell-note">{l.description}</small>
                        </td>
                        <td className="numeric">
                          {l.debit_minor === '0' ? '—' : money(l.debit_minor, currency, false)}
                        </td>
                        <td className="numeric">
                          {l.credit_minor === '0' ? '—' : money(l.credit_minor, currency, false)}
                        </td>
                        <td className="numeric strong">{balanceText(l.balance_minor, currency)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <Empty title="No movements in this period">
                Choose another account or adjust the date range.
              </Empty>
            )}
            <div className="panel-toolbar">
              <button
                className="button secondary"
                disabled={!offset}
                onClick={() => setOffset(Math.max(0, offset - 50))}
              >
                Previous
              </button>
              <span>Page {offset / 50 + 1}</span>
              <button
                className="button secondary"
                disabled={offset + 50 >= (data.line_count || 0)}
                onClick={() => setOffset(offset + 50)}
              >
                Next
              </button>
            </div>
          </section>
        </>
      )}
      {loading && !data && <div className="panel empty">Loading ledger…</div>}
    </>
  );
}
function TrialView({ state, onError }: { state: Snapshot; onError: (e: string) => void }) {
  const [date, setDate] = useState('');
  const [data, setData] = useState(state.trial_balance);
  const [hideZero, setHideZero] = useState(false);
  useEffect(() => {
    let active = true;
    request<TrialBalance>('trial_balance', { as_of: date || null })
      .then((v) => {
        if (active) setData(v);
      })
      .catch((e) => onError(String(e.message || e)));
    return () => {
      active = false;
    };
  }, [date, state.journal_count]);
  const rows = data.rows.filter((r) => !hideZero || r.net_debit_minor !== '0');
  const currency = state.company!.currency;
  return (
    <>
      <section className="panel filter-panel">
        <label>
          As of date
          <input type="date" value={date} onChange={(e) => setDate(e.target.value)} />
        </label>
        <span className="filter-description">
          {date ? `All entries through ${displayDate(date)}` : 'All posted entries'}
        </span>
        <label className="checkbox-label">
          <input
            type="checkbox"
            checked={hideZero}
            onChange={(e) => setHideZero(e.target.checked)}
          />
          Hide zero balances
        </label>
        <button
          className="button secondary"
          onClick={() =>
            download(
              `trial-balance-${date || 'all'}.csv`,
              csv([
                ['Code', 'Account', 'Type', `Debit (${currency})`, `Credit (${currency})`],
                ...rows.map((r) => [
                  r.account.code,
                  r.account.name,
                  r.account.kind,
                  money(r.debit_minor, currency, false),
                  money(r.credit_minor, currency, false),
                ]),
                [
                  'TOTAL',
                  '',
                  '',
                  money(data.debit_minor, currency, false),
                  money(data.credit_minor, currency, false),
                ],
              ]),
              'text/csv',
            )
          }
        >
          <Icon name="download" size={16} />
          Export CSV
        </button>
      </section>
      <section className="panel">
        <div className="panel-title">
          <div>
            <h3>Trial balance</h3>
            <p>Closing account balances · {currency}</p>
          </div>
          <Badge>
            <Icon name="check" size={13} />
            {data.balanced ? 'In balance' : 'Out of balance'}
          </Badge>
        </div>
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th>Account code</th>
                <th>Account name</th>
                <th>Type</th>
                <th className="numeric">Debit</th>
                <th className="numeric">Credit</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((r) => (
                <tr key={r.account.id}>
                  <td className="account-code">{r.account.code}</td>
                  <td className="strong">{r.account.name}</td>
                  <td>
                    <span className={`kind-dot ${r.account.kind}`} />
                    {kindName[r.account.kind]}
                  </td>
                  <td className="numeric">
                    {r.debit_minor === '0' ? '—' : money(r.debit_minor, currency, false)}
                  </td>
                  <td className="numeric">
                    {r.credit_minor === '0' ? '—' : money(r.credit_minor, currency, false)}
                  </td>
                </tr>
              ))}
            </tbody>
            <tfoot>
              <tr>
                <td colSpan={3}>Total closing balances</td>
                <td className="numeric">{money(data.debit_minor, currency)}</td>
                <td className="numeric">{money(data.credit_minor, currency)}</td>
              </tr>
            </tfoot>
          </table>
        </div>
        <div className={`trial-equation ${data.balanced ? '' : 'unbalanced'}`}>
          <Icon name="check" size={20} />
          <div>
            <strong>
              {data.balanced ? 'Debits equal credits.' : 'The trial balance is out of balance.'}
            </strong>
            <p>
              Difference: {money(BigInt(data.debit_minor) - BigInt(data.credit_minor), currency)}
            </p>
          </div>
        </div>
      </section>
    </>
  );
}
function Backups({
  state,
  busy,
  exportBackup,
  mutate,
  onError,
}: {
  state: Snapshot;
  busy: boolean;
  exportBackup: () => void;
  mutate: Mutation;
  onError: (e: string) => void;
}) {
  const file = useRef<HTMLInputElement>(null);
  const [contents, setContents] = useState('');
  const [preview, setPreview] = useState<BackupPreview | null>(null);
  const [reading, setReading] = useState(false);
  const inspect = async (input: File) => {
    if (input.size > 20 * 1024 * 1024) {
      onError('Choose a backup smaller than 20 MB.');
      return;
    }
    setReading(true);
    try {
      const text = await input.text();
      const value = await request<BackupPreview>('preview_backup', { contents: text });
      setContents(text);
      setPreview(value);
    } catch (e) {
      onError(String(e instanceof Error ? e.message : e));
    } finally {
      setReading(false);
      if (file.current) file.current.value = '';
    }
  };
  return (
    <>
      <div className="backup-grid">
        <section className="panel backup-card">
          <span className="backup-icon">
            <Icon name="download" size={28} />
          </span>
          <h3>Take your books with you</h3>
          <p>
            Save your company, chart of accounts, and complete journal history in a portable backup.
          </p>
          <div className="backup-stat">
            <span>Included in this backup</span>
            <strong>
              {state.accounts.length} accounts · {state.journal_count} journal entries
            </strong>
          </div>
          <button className="button primary" disabled={busy} onClick={exportBackup}>
            <Icon name="download" size={17} />
            {busy ? 'Preparing backup…' : 'Save backup'}
          </button>
        </section>
        <section className="panel backup-card">
          <span className="backup-icon pale">
            <Icon name="reverse" size={28} />
          </span>
          <h3>Restore a previous backup</h3>
          <p>
            Select an Open Ledger backup. You can review the company and entry counts before
            replacing your current books.
          </p>
          <div className="backup-stat">
            <span>Recovery protection</span>
            <strong>Your current books are saved before restore.</strong>
          </div>
          <input
            type="file"
            accept=".json,application/json"
            ref={file}
            className="visually-hidden"
            aria-label="Choose backup file"
            onChange={(e) => {
              if (e.target.files?.[0]) void inspect(e.target.files[0]);
            }}
          />
          <button
            className="button secondary"
            disabled={busy || reading}
            onClick={() => file.current?.click()}
          >
            <Icon name="file" size={17} />
            {reading ? 'Checking backup…' : 'Choose backup'}
          </button>
        </section>
      </div>
      <section className="panel backup-guide">
        <Icon name="shield" size={25} />
        <div>
          <h3>Make backup a habit</h3>
          <p>
            Keep another copy on a different drive or in your preferred storage. Restore checks
            account references, balanced journals, and reversal links before replacing any records.
          </p>
        </div>
      </section>
      {preview && (
        <ModalFrame
          title="Restore these books?"
          eyebrow="REVIEW BACKUP"
          close={() => {
            if (!busy) setPreview(null);
          }}
        >
          <div className="restore-details">
            <strong>{preview.company_name}</strong>
            <p>
              {preview.currency} · {preview.account_count} accounts · {preview.journal_count}{' '}
              entries
            </p>
            <small>Exported {new Date(preview.exported_at).toLocaleString()}</small>
          </div>
          <p className="restore-warning">
            This replaces the current books for <strong>{state.company!.name}</strong>. A recovery
            backup of your current records will be saved first.
          </p>
          <div className="modal-actions">
            <button className="button secondary" disabled={busy} onClick={() => setPreview(null)}>
              Cancel
            </button>
            <button
              className="button primary"
              disabled={busy}
              onClick={async () => {
                if (
                  await mutate(
                    'restore_backup',
                    { contents },
                    'Backup restored. Previous books saved for recovery.',
                  )
                )
                  setPreview(null);
              }}
            >
              {busy ? 'Restoring…' : 'Restore backup'}
            </button>
          </div>
        </ModalFrame>
      )}
    </>
  );
}
function JournalComposer({
  state,
  busy,
  close,
  mutate,
}: {
  state: Snapshot;
  busy: boolean;
  close: () => void;
  mutate: Mutation;
}) {
  const [date, setDate] = useState(today());
  const [reference, setReference] = useState('');
  const [description, setDescription] = useState('');
  const [key] = useState(crypto.randomUUID());
  const [lines, setLines] = useState<{ account_id: number; side: Side; amount: string }[]>([
    { account_id: 0, side: 'debit', amount: '' },
    { account_id: 0, side: 'credit', amount: '' },
  ]);
  const debit = lines
    .filter((l) => l.side === 'debit')
    .reduce((sum, l) => sum + (parseMinor(l.amount) || 0n), 0n);
  const credit = lines
    .filter((l) => l.side === 'credit')
    .reduce((sum, l) => sum + (parseMinor(l.amount) || 0n), 0n);
  const balanced = debit === credit && debit > 0n;
  const valid = balanced && lines.every((l) => l.account_id && (parseMinor(l.amount) || 0n) > 0n);
  const update = (i: number, change: object) =>
    setLines(lines.map((l, index) => (index === i ? { ...l, ...change } : l)));
  const fill = () => {
    const rest = lines
      .slice(0, -1)
      .reduce((sum, l) => sum + (l.side === 'debit' ? 1n : -1n) * (parseMinor(l.amount) || 0n), 0n);
    if (rest)
      update(lines.length - 1, {
        side: rest > 0n ? 'credit' : 'debit',
        amount: decimal(rest > 0n ? rest : -rest),
      });
  };
  return (
    <ModalFrame title="New journal entry" eyebrow="RECORD A TRANSACTION" close={close} wide>
      <form
        onSubmit={async (e) => {
          e.preventDefault();
          if (
            await mutate(
              'post_journal',
              { date, reference, description, request_id: key, lines },
              'Journal entry posted.',
            )
          )
            close();
        }}
      >
        <div className="form-columns">
          <label>
            Posting date
            <input type="date" required value={date} onChange={(e) => setDate(e.target.value)} />
          </label>
          <label>
            Reference <span className="muted">optional</span>
            <input
              value={reference}
              maxLength={64}
              placeholder="Assigned automatically if empty"
              onChange={(e) => setReference(e.target.value)}
            />
          </label>
        </div>
        <label>
          Description
          <input
            required
            maxLength={1000}
            value={description}
            placeholder="What is this transaction for?"
            onChange={(e) => setDescription(e.target.value)}
          />
        </label>
        <div className="posting-header">
          <span>POSTING LINES</span>
          <span>{state.company!.currency}</span>
        </div>
        <div className="posting-labels">
          <span>Account</span>
          <span>Side</span>
          <span>Amount</span>
          <span />
        </div>
        <div className="posting-lines">
          {lines.map((line, i) => (
            <div className="posting-row" key={i}>
              <select
                required
                aria-label={`Account for line ${i + 1}`}
                value={line.account_id}
                onChange={(e) => update(i, { account_id: Number(e.target.value) })}
              >
                <option value={0}>Choose account…</option>
                {state.accounts.map((a) => (
                  <option value={a.id} key={a.id}>
                    {a.code} · {a.name}
                  </option>
                ))}
              </select>
              <select
                aria-label={`Side for line ${i + 1}`}
                value={line.side}
                onChange={(e) => update(i, { side: e.target.value })}
              >
                <option value="debit">Debit</option>
                <option value="credit">Credit</option>
              </select>
              <input
                aria-label={`Amount for line ${i + 1}`}
                required
                inputMode="decimal"
                placeholder="0.00"
                value={line.amount}
                onChange={(e) => update(i, { amount: e.target.value })}
              />
              <button
                type="button"
                className="icon-button"
                aria-label={`Remove line ${i + 1}`}
                disabled={lines.length <= 2}
                onClick={() => setLines(lines.filter((_, index) => index !== i))}
              >
                <Icon name="close" size={15} />
              </button>
            </div>
          ))}
        </div>
        <div className="posting-tools">
          <button
            type="button"
            className="text-button"
            disabled={lines.length >= 500}
            onClick={() => setLines([...lines, { account_id: 0, side: 'debit', amount: '' }])}
          >
            <Icon name="plus" size={16} />
            Add line
          </button>
          <button type="button" className="text-button" onClick={fill}>
            Balance last line
          </button>
        </div>
        <div className={`posting-totals ${balanced ? 'balanced' : ''}`}>
          <div>
            <span>Total debits</span>
            <strong>{money(debit, state.company!.currency)}</strong>
          </div>
          <div>
            <span>Total credits</span>
            <strong>{money(credit, state.company!.currency)}</strong>
          </div>
          <div>
            <span>{balanced ? 'In balance' : 'Difference'}</span>
            <strong>
              {balanced ? (
                <Icon name="check" />
              ) : (
                money(debit > credit ? debit - credit : credit - debit, state.company!.currency)
              )}
            </strong>
          </div>
        </div>
        <p className="form-hint">
          Posted entries remain in your accounting trail. Use a reversal to correct an entry.
        </p>
        <div className="modal-actions">
          <button type="button" className="button secondary" disabled={busy} onClick={close}>
            Cancel
          </button>
          <button className="button primary" disabled={busy || !valid}>
            {busy ? 'Posting…' : 'Post journal entry'}
            <Icon name="check" size={16} />
          </button>
        </div>
      </form>
    </ModalFrame>
  );
}
function AccountComposer({
  busy,
  close,
  mutate,
}: {
  busy: boolean;
  close: () => void;
  mutate: Mutation;
}) {
  const [code, setCode] = useState('');
  const [name, setName] = useState('');
  const [kind, setKind] = useState<Kind>('asset');
  return (
    <ModalFrame title="Add an account" eyebrow="CHART OF ACCOUNTS" close={close}>
      <form
        onSubmit={async (e) => {
          e.preventDefault();
          if (await mutate('create_account', { code, name, kind }, 'Account added.')) close();
        }}
      >
        <div className="form-columns">
          <label>
            Account code
            <input
              required
              maxLength={24}
              value={code}
              placeholder="e.g. 5100"
              onChange={(e) => setCode(e.target.value)}
            />
          </label>
          <label>
            Type
            <select value={kind} onChange={(e) => setKind(e.target.value as Kind)}>
              {kinds.map((k) => (
                <option key={k} value={k}>
                  {kindName[k]}
                </option>
              ))}
            </select>
          </label>
        </div>
        <label>
          Account name
          <input
            required
            maxLength={120}
            value={name}
            placeholder="e.g. Office supplies"
            onChange={(e) => setName(e.target.value)}
          />
        </label>
        <p className="form-hint">
          Account definitions stay fixed once they have posted transactions.
        </p>
        <div className="modal-actions">
          <button className="button secondary" type="button" disabled={busy} onClick={close}>
            Cancel
          </button>
          <button className="button primary" disabled={busy}>
            {busy ? 'Adding…' : 'Add account'}
          </button>
        </div>
      </form>
    </ModalFrame>
  );
}
function ReverseComposer({
  journal,
  busy,
  close,
  mutate,
}: {
  journal: Journal;
  busy: boolean;
  close: () => void;
  mutate: Mutation;
}) {
  const [date, setDate] = useState(today() < journal.date ? journal.date : today());
  const [reason, setReason] = useState('');
  const [key] = useState(crypto.randomUUID());
  return (
    <ModalFrame title={`Reverse ${journal.reference}`} eyebrow="CORRECT A POSTING" close={close}>
      <form
        onSubmit={async (e) => {
          e.preventDefault();
          if (
            await mutate(
              'reverse_journal',
              { id: journal.id, date, reason, request_id: key },
              'Reversal posted. The original entry is preserved.',
            )
          )
            close();
        }}
      >
        <div className="restore-details">
          <strong>{journal.description}</strong>
          <p>Original date: {displayDate(journal.date)}</p>
        </div>
        <label>
          Reversal date
          <input
            type="date"
            required
            min={journal.date}
            value={date}
            onChange={(e) => setDate(e.target.value)}
          />
        </label>
        <label>
          Reason
          <input
            required
            maxLength={500}
            value={reason}
            placeholder="Explain the correction"
            onChange={(e) => setReason(e.target.value)}
          />
        </label>
        <p className="form-hint">
          A new entry will swap the original debits and credits. The original journal remains in
          your records.
        </p>
        <div className="modal-actions">
          <button type="button" className="button secondary" disabled={busy} onClick={close}>
            Cancel
          </button>
          <button className="button primary" disabled={busy}>
            {busy ? 'Posting reversal…' : 'Post reversal'}
          </button>
        </div>
      </form>
    </ModalFrame>
  );
}
function JournalDetails({
  journal,
  currency,
  close,
  reverse,
}: {
  journal: Journal;
  currency: string;
  close: () => void;
  reverse: () => void;
}) {
  return (
    <ModalFrame title={journal.reference} eyebrow="POSTED JOURNAL" close={close} wide>
      <div className="journal-detail-heading">
        <div>
          <h3>{journal.description}</h3>
          <p>{displayDate(journal.date)}</p>
        </div>
        <Badge neutral={Boolean(journal.reversed_by || journal.reversal_of)}>
          {journal.reversed_by ? 'Reversed' : journal.reversal_of ? 'Reversal' : 'Posted'}
        </Badge>
      </div>
      <div className="table-scroll">
        <table>
          <thead>
            <tr>
              <th>Account</th>
              <th className="numeric">Debit</th>
              <th className="numeric">Credit</th>
            </tr>
          </thead>
          <tbody>
            {journal.lines.map((l, i) => (
              <tr key={i}>
                <td>
                  <strong>{l.account_name}</strong>
                  <small className="cell-note">{l.account_code}</small>
                </td>
                <td className="numeric">
                  {l.side === 'debit' ? money(l.amount_minor, currency) : '—'}
                </td>
                <td className="numeric">
                  {l.side === 'credit' ? money(l.amount_minor, currency) : '—'}
                </td>
              </tr>
            ))}
          </tbody>
          <tfoot>
            <tr>
              <td>Total</td>
              <td className="numeric">{money(journal.total_minor, currency)}</td>
              <td className="numeric">{money(journal.total_minor, currency)}</td>
            </tr>
          </tfoot>
        </table>
      </div>
      <p className="form-hint">
        Recorded {new Date(journal.created_at).toLocaleString()}
        {journal.reversal_of ? ` · Reverses journal #${journal.reversal_of}` : ''}
        {journal.reversed_by ? ` · Reversed by journal #${journal.reversed_by}` : ''}
      </p>
      <div className="modal-actions">
        {!journal.reversed_by && !journal.reversal_of && (
          <button className="button secondary" onClick={reverse}>
            <Icon name="reverse" size={16} />
            Reverse entry
          </button>
        )}
        <button className="button primary" onClick={close}>
          Done
        </button>
      </div>
    </ModalFrame>
  );
}

createRoot(document.getElementById('root')!).render(<App />);
