# Accounting workflows and worked examples

These rules define the general accounting implementation. They need an independent accounting review before a stable release.

## Sales and purchases

Create a customer mapped to an asset receivable account, or a supplier mapped to a liability payable account. Multiple parties may share a control account; invoice IDs maintain the subledger. Contacts and addresses are optional. Party mappings become immutable. Use a new party record for corrected mappings.

A service sale for 1,000.00 debits receivables 1,000.00 and credits sales income 1,000.00. A 400.00 receipt debits bank 400.00 and credits receivables 400.00. The invoice and receivable ledger both show 600.00 remaining. A 700.00 service purchase debits expense and credits payables; a 250.00 payment debits payables and credits bank, leaving 450.00 payable.

Amounts are positive decimal strings, with at most two fractional digits. Invoice quantities have up to three fractional digits. A line extension rounds half up to the nearest minor unit; invoice totals sum the rounded extensions. Document and journal writes, numbering, and stock changes share one SQLite transaction. A request identifier with the same payload retries the original result; reusing it with a different payload is rejected. Receipts cannot exceed the outstanding amount, precede their invoice, or precede the latest settlement/reversal for that invoice.

Due dates cannot precede invoice dates. Balances as of a date include only documents and reversals effective by that date. Unpaid invoices become overdue after the due date. Tax rates and eligibility are not inferred. Optional supported India GST is specified separately in [INDIA_GST.md](INDIA_GST.md); exchange rates and statutory filing remain unsupported.

Drafts are saved explicitly on this device, separately from the accounting database. Drafts are not included in backups. Review displays the document before posting. Posted documents cannot be edited. Service invoice reversals require linked settlements to be reversed first and cannot precede those reversals. View the document's Journal to reverse it.

## Inventory

Create dedicated inventory asset accounts and cost-of-sales expense accounts, then create items with a fixed unit. Use purchase invoices or stock receipts to introduce stock. Stock issues and sales consume it. Quantities have three decimal places; fractional units are supported but unit conversions, batches, serial numbers, and alternate valuations are not.

Valuation is perpetual weighted average, with half-up cost rounding at each issue. Exhausting an item consumes the entire remaining value, preventing leftover cents. Buying 10 units at 10.00 and 10 at 20.00 gives 20 units costing 300.00. Selling 4 at 25.00 recognizes revenue 100.00, cost of sales 60.00, and profit 40.00; remaining stock is 16 units valued at 240.00. The inventory account also holds 240.00.

Negative stock is rejected. Stock documents cannot be backdated before the latest stock movement. Ordinary journals cannot post to an inventory asset account. Stock receipt adjustments debit inventory and credit the selected income, expense, or equity offset. Issue adjustments debit the item's expense account and credit inventory at carrying cost. Correct inventory with an opposite adjustment. Linked quantity returns and full cancellation of remaining invoice quantities preserve the original sale/purchase and derive opposite stock movements. Sales returns restore original issue cost; purchase returns remove current weighted average cost and post any price variance to the item expense account. See [returns and refunds](RETURNS.md) for examples, constraints and linked refund accounting. Reversing inventory or return journals independently is prohibited.

## Financial periods and opening balances

Record opening balances as a balanced journal at the financial-year start, with the appropriate equity offset. Periods must be consecutive from the company's fiscal start. Closing clears income/expense balances into the selected equity account. The transfer is immutable, and new journals or reversals cannot post into any closed date. Closing an empty-earnings period still locks the dates.

Asset, liability, and equity balances carry forward through the same continuous ledger. A date-filtered ledger computes its opening balance from all earlier postings. Profit and loss excludes closing transfers, so earned income/expenses remain visible after closing. The balance sheet includes current unclosed earnings in addition to posted equity. In the example of 1,000.00 capital, 300.00 income and 100.00 expense, closing yields bank assets and equity of 1,200.00; period profit remains 200.00.

Closing is intentionally final in this implementation. Review reports and export a backup first. Restoring a prior backup is the recovery mechanism; it replaces all subsequent work as well, with a recovery backup preserved.

## Bank reconciliation

Choose an asset bank account, statement dates, its actual opening/closing balances, and the ledger lines cleared on that statement. Earlier outstanding transactions may clear later. Opening plus selected debit movements minus selected credit movements must equal closing. Lines cannot be cleared twice; subsequent statements must be later and their opening must match the previous statement's closing. Statements and cleared-line assignments are immutable. Corrections are new adjusting accounting entries; statement reopening is not implemented.

Reconciliation does not infer missing bank transactions. Post charges, interest, or other adjustments explicitly, then clear their bank lines. The interface shows book balance, selected movements, and outstanding movements. Historical reconciliation remains auditable even when a new journal reverses a previously cleared transaction.
