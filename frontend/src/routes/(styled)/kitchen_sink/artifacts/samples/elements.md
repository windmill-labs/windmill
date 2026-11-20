# Heading one

A paragraph with **bold**, _italic_, **_both_**, ~~struck~~, `inline code`, a [link](https://www.windmill.dev), and <kbd>Cmd</kbd> + <kbd>K</kbd>. Long identifiers wrap: `f/a_long_folder_name/with_a_script_path_that_keeps_going_and_going`.

## Heading two

Body text under a section heading. A second sentence shows the line length and the leading on a paragraph that runs over several lines, which is where the measure and the line height matter most.

### Heading three

#### Heading four

##### Heading five

###### Heading six

## Lists

- An unordered item
- An item with a nested list
  - Nested item
  - Another nested item
    - Third level
- Last item

1. Ordered item
2. Ordered item with a nested list
   1. Nested ordered item
   2. Another one
3. Last ordered item

- [x] A checked task
- [ ] An open task
- [ ] An open task with `code`

## Quote

> A quote can run over several lines. It keeps the measure of the text around it and sits back from it.
>
> - It can hold a list
> - with two items

## Callouts

> [!NOTE]
> Useful information the reader should notice.

> [!TIP]
> A better way to do something.

> [!IMPORTANT]
> Something the reader needs to know to succeed.

> [!WARNING]
> Something that needs attention right away.

> [!CAUTION]
> The risky outcome of an action.

## Code

```ts
export async function main(workspace: string, limit = 100) {
	const jobs = await wmill.listJobs({ workspace, perPage: limit })
	return jobs.filter((job) => job.success === false)
}
```

```sql
select path, count(*) as runs
from v2_job
where created_at > now() - interval '1 day'
group by path
order by runs desc;
```

```bash
wmill sync pull --yes && wmill script push f/billing/sync_page.py
```

```
A fence without a language renders as plain text.
```

## Table

| Left                                                      | Center | Right |
| :-------------------------------------------------------- | :----: | ----: |
| Text                                                      |  Text  | 1,024 |
| A longer cell that wraps when the column runs out of room |  Text  |     2 |
| Text                                                      | `code` |  3.14 |

## Wide table

A table wider than the column scrolls inside its frame.

| Run        | Script                      | Workspace | Trigger   | Worker      | Queued | Started  |  Duration | Memory | Status  | Retries | Tags            | Owner |
| :--------- | :-------------------------- | :-------- | :-------- | :---------- | -----: | :------- | --------: | -----: | :------ | ------: | :-------------- | :---- |
| `0192f3a1` | `f/billing/sync_invoices`   | finance   | Schedule  | worker-eu-1 |  0.4 s | 09:00:02 |  1 m 12 s | 212 MB | Success |       0 | python, billing | alice |
| `0192f3a7` | `f/billing/send_reminders`  | finance   | Webhook   | worker-eu-2 |  1.2 s | 09:01:15 |      18 s |  96 MB | Success |       0 | bun             | alice |
| `0192f3b2` | `f/ops/rotate_keys`         | ops       | Manual    | worker-us-1 |  0.1 s | 09:03:40 |       4 s |  48 MB | Failed  |       2 | bash            | bob   |
| `0192f3c9` | `f/data/nightly_export`     | data      | Schedule  | worker-us-3 | 12.8 s | 09:05:00 | 14 m 03 s | 1.4 GB | Running |       0 | duckdb, s3      | carol |
| `0192f3d4` | `f/data/refresh_dashboards` | data      | Flow step | worker-eu-1 |  0.6 s | 09:07:21 |  2 m 41 s | 384 MB | Success |       1 | python          | carol |

## Details

<details>
<summary>A collapsed section</summary>

The content shows when the summary is clicked.

</details>

---

A footnote reference.[^note]

[^note]: The footnote text, at the end of the document.
