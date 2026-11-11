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

## Details

<details>
<summary>A collapsed section</summary>

The content shows when the summary is clicked.

</details>

---

A footnote reference.[^note]

[^note]: The footnote text, at the end of the document.
