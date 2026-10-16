# Linked invoice returns and refunds

From **Invoices & settlements**, choose **Return / cancel** on the original sales or purchase invoice. Enter a date, reason and returned quantities. **Cancel remaining invoice** selects every remaining quantity. Review the derived accounting and stock amounts, then post the immutable linked note. A cancellation preserves the original invoice and all earlier settlements.

Sales returns create `credit_note` documents. Purchase returns create `debit_note` documents, displayed as **Purchase return note**: this is the buyer's commercial adjustment, not a supplier-issued statutory GST debit note. A regular taxable adjustment requires explicit review of eligibility and supporting supplier/recipient documents. Statutory filing remains outside the implemented GST scope; see [INDIA_GST.md](INDIA_GST.md).

## Amounts and stock

Returns select original zero-based line indices and quantities, with at most three decimal places. Descriptions, prices, account mappings and tax rates come from the original invoice. Previously returned quantities reduce the available quantity. Service quantities represent the cancelled fraction of the original service. Monetary-only allowances, price corrections without a quantity return, and replacement goods on the same note need separate workflows.

Each partial credit is the difference between rounded cumulative proportions of the original line's taxable amount and each original tax component. A partial quantity reserves at least one minor unit of each nonzero original component for the final quantity. This prevents an early rounded credit from exhausting the money while leaving an unpostable physical return. Three one-unit returns against a three-unit line sum exactly to that line, including residual paise. Amounts are integer minor units throughout. A note whose gross amount rounds to zero is rejected; combine a larger quantity in one note.

A sales stock return restores its proportional original issue cost: debit inventory, credit cost of sales. It also debits original sales income and any reviewed output GST, and credits customer receivables by the gross credit. Historical sale costs are preserved even if later purchases changed the weighted average.

A purchase stock return removes inventory at its current weighted average carrying cost and debits payable by the original credited gross amount. Reviewed input GST is credited separately. The difference between credited taxable purchase price plus its original nonrecoverable GST and removed inventory cost posts to the item's expense account. For example, buying 10 units at 10.00 and 10 at 20.00 yields a 15.00 average. Returning two units from the first purchase credits inventory 30.00, debits payable 20.00 and debits purchase-return variance to the mapped expense account 10.00. Quantity and inventory value remain aligned with the ledger. This policy needs independent accounting review before stable use.

All returns follow the original invoice and its latest linked event. Stock returns also follow the latest stock movement across the books. Closed periods, over-returns and negative stock are rejected atomically. Notes and original invoices with linked notes cannot have their journals reversed separately; correct an erroneous return with a reviewed replacement invoice. Other stock adjustments retain their existing correction workflow.

## Settlements

Invoice balance equals original gross amount minus active net settlements minus linked credits. An excess receipt/payment after a return appears as **Refund due**. **Record refund** creates a linked customer or supplier refund; it cannot exceed that credit. Customer refunds debit receivables and credit bank; supplier refunds debit bank and credit payables. All are dated and idempotent. Refunds must be reversed before reversing the cash settlement that funded them.

For a 300.00 sale paid in full, a 100.00 credit creates a 100.00 customer refund due. Refunding it leaves the invoice settled, sales at 200.00 and bank at 200.00. As-of balances include credits, payments, refunds and reversals only on their effective dates.

## API and recovery

`returnable_invoice` accepts `{ "id": 1 }` and returns the original invoice and remaining quantities as integer milliunit strings. `preview_document` validates against a separate copy of current books and returns a derived document and journal without changing live records. Notes use the original `invoice_id`, matching `party_id`, `return_lines: [{ "line": 0, "quantity": "1" }]`, date, reason and unique `request_id`; settlement fields and ordinary invoice lines must be empty. Regular taxable GST notes additionally require `gst_adjustment_reviewed: true`.

The interface submits `reviewed_postings` from the preview. Posting recalculates and rejects the operation if quantities or current costs changed since review. A retry with the same request ID and payload returns the original document. Backup version 6 preserves links, return details, review records, refunds and derived stock movements. Restore recalculates these from the original records and rejects altered amounts, quantities or valuations before replacing books. Version 1, 2, 3, 4 and 5 backups remain supported, with their original numbering sequences upgraded to ten sequences.
