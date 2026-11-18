# Migrate invoice sync to a flow

The nightly `f/billing/sync_invoices` script times out on months with more than 30,000 invoices. This plan splits it into a flow with one step per page of results, so a failed page can be retried on its own.

## Goals

- Finish the March backfill (41,200 invoices) in under 20 minutes
- Retry a failed page without re-running the whole sync
- Keep the Stripe resource, the schedule and its run history unchanged

## Approach

1. Add `f/billing/list_invoice_pages`, which returns the page cursors for a month
2. Run `f/billing/sync_page` in a **for-loop** step with parallelism 4
3. Move the Postgres upsert into the loop body, keyed on `stripe_invoice_id`
4. Point the existing schedule at the new flow

> [!NOTE]
> The schedule keeps its path, so the existing run history stays attached to it.

### Expected cost per run

| Step       |       Duration |    Jobs |
| :--------- | -------------: | ------: |
| List pages |            4 s |       1 |
| Sync page  |     2 min 10 s |      83 |
| Upsert     |           35 s |      83 |
| **Total**  | **2 min 49 s** | **167** |

### The loop body

```python
def main(cursor: str, stripe: dict):
    client = stripe_client(stripe)
    page = client.invoices.list(starting_after=cursor, limit=500)
    return [invoice.to_dict() for invoice in page.data]
```

## Risks

> [!WARNING]
> Stripe allows 100 read requests per second. Parallelism above 8 hits that limit during the backfill.

- The upsert holds a row lock per invoice, so two overlapping runs would block each other
- The old script stays deployed until one full month has run on the flow

## Open questions

- [x] Confirm the 500-per-page limit with the Stripe API docs
- [ ] Decide whether to keep the old script as a fallback
- [ ] Choose who receives the failure alert

---

Rollback: point the schedule back at `f/billing/sync_invoices`. The [scheduling docs](https://www.windmill.dev/docs/core_concepts/scheduling) cover the procedure.
