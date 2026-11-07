import { parseDiffFromFile, type FileDiffMetadata } from '@pierre/diffs'
import type { ToolCodeDiff } from '$lib/components/copilot/chat/shared'
import { pierreLang } from '$lib/components/pierre/pierre'

export type ComparisonCase = {
	id: string
	label: string
	filename: string
	diff: ToolCodeDiff
}

export function caseFileDiff(c: ComparisonCase): FileDiffMetadata {
	return parseDiffFromFile(
		{
			name: c.filename,
			contents: c.diff.before,
			lang: pierreLang(c.diff.beforeLang ?? c.diff.lang)
		},
		{ name: c.filename, contents: c.diff.after, lang: pierreLang(c.diff.lang) }
	)
}

const stableLines = Array.from(
	{ length: 14 },
	(_, index) => `	const stableValue${index + 1} = getStableValue(${index + 1})`
)

/** A TypeScript module of `functions` functions, with every `editEvery`th one changed in `after`. */
function largeModule(functions: number, editEvery: number): { before: string; after: string } {
	const before: string[] = []
	const after: string[] = []
	for (let i = 0; i < functions; i++) {
		const body = [
			`export async function handler${i}(input: { id: string; count: number }) {`,
			`	const result = await fetchRecord(input.id)`,
			`	if (!result.ok) {`,
			`		throw new Error(\`record \${input.id} not found\`)`,
			`	}`,
			`	return { ...result.value, total: input.count * ${i} }`,
			`}`,
			``
		]
		before.push(...body)
		if (i % editEvery === 0) {
			after.push(
				`export async function handler${i}(input: { id: string; count: number; dryRun?: boolean }) {`,
				`	const result = await fetchRecord(input.id, { cache: true })`,
				`	if (!result.ok || input.dryRun) {`,
				`		return { skipped: true }`,
				`	}`,
				`	return { ...result.value, total: input.count * ${i + 1} }`,
				`}`,
				``
			)
		} else {
			after.push(...body)
		}
	}
	return { before: before.join('\n'), after: after.join('\n') }
}

const large = largeModule(1250, 25)

const longLine = (n: number) =>
	`const payload = ${JSON.stringify(
		Object.fromEntries(Array.from({ length: n }, (_, i) => [`field_${i}`, `value number ${i}`]))
	)}`

export const cases: ComparisonCase[] = [
	{
		id: 'typescript-replacement',
		label: 'TypeScript replacement',
		filename: 'greeting.ts',
		diff: {
			lang: 'typescript',
			before: `export function greeting(name: string) {
	return \`Hello, \${name}\`
}`,
			after: `export function greeting(name: string) {
	const message = \`Hello, \${name}!\`
	return message
}`
		}
	},
	{
		id: 'long-unchanged-sections',
		label: 'Long unchanged sections',
		filename: 'summary.ts',
		diff: {
			lang: 'typescript',
			before: [
				'export function buildSummary() {',
				...stableLines,
				'\treturn formatSummary("draft")',
				'}'
			].join('\n'),
			after: [
				'export function buildSummary() {',
				...stableLines,
				'\treturn formatSummary("published")',
				'}'
			].join('\n')
		}
	},
	{
		id: 'json-structure',
		label: 'JSON structure',
		filename: 'schedule.json',
		diff: {
			lang: 'json',
			before: `{
	"name": "daily-report",
	"schedule": "0 9 * * 1-5",
	"enabled": false
}`,
			after: `{
	"name": "daily-report",
	"schedule": "0 9 * * 1-5",
	"timezone": "Europe/Paris",
	"enabled": true
}`
		}
	},
	{
		id: 'python-add-remove',
		label: 'Python additions and removals',
		filename: 'notify.py',
		diff: {
			lang: 'python',
			before: `def notify(users):
	for user in users:
		send_email(user, "Weekly update")

	return len(users)`,
			after: `def notify(users):
	active_users = [user for user in users if user.active]
	for user in active_users:
		send_email(user, "Your weekly update")

	return len(active_users)`
		}
	},
	{
		id: 'flow-yaml',
		label: 'Flow YAML',
		filename: 'flow.yaml',
		diff: {
			lang: 'yaml',
			before: `summary: Sync customers
value:
  modules:
    - id: a
      value:
        type: script
        path: f/sync/fetch_customers
        input_transforms:
          since:
            type: javascript
            expr: flow_input.since
    - id: b
      value:
        type: rawscript
        language: python3
        content: |
          def main(rows):
              return [r for r in rows if r["active"]]
schema:
  properties:
    since:
      type: string
`,
			after: `summary: Sync active customers
value:
  modules:
    - id: a
      value:
        type: script
        path: f/sync/fetch_customers
        input_transforms:
          since:
            type: javascript
            expr: flow_input.since ?? "2024-01-01"
      retry:
        constant:
          attempts: 3
          seconds: 10
    - id: b
      value:
        type: rawscript
        language: python3
        content: |
          def main(rows):
              return [r for r in rows if r["active"] and not r.get("deleted")]
schema:
  properties:
    since:
      type: string
      format: date
`
		}
	},
	{
		id: 'long-lines',
		label: 'Long lines',
		filename: 'payload.ts',
		diff: {
			lang: 'typescript',
			before: [longLine(40), 'export default payload'].join('\n'),
			after: [longLine(41), 'export default payload'].join('\n')
		}
	},
	{
		id: 'new-file',
		label: 'New file',
		filename: 'new.py',
		diff: {
			lang: 'python',
			before: '',
			after: `import wmill


def main(name: str = "world"):
    wmill.set_state({"last": name})
    return f"Hello {name}"
`
		}
	},
	{
		id: 'deleted-file',
		label: 'Deleted file',
		filename: 'old.sql',
		diff: {
			lang: 'sql',
			before: `-- $1 email
SELECT id, email, created_at
FROM users
WHERE email = $1
ORDER BY created_at DESC;
`,
			after: ''
		}
	},
	{
		id: 'large',
		label: `Large (${large.before.split('\n').length.toLocaleString()} lines)`,
		filename: 'handlers.ts',
		diff: { lang: 'typescript', ...large }
	}
]

export const MERGE_CONFLICT_SAMPLE = `import * as wmill from 'windmill-client'

export async function main(customerId: string) {
<<<<<<< HEAD
	const customer = await wmill.getResource('u/admin/crm')
	return fetchCustomer(customer, customerId, { retries: 3 })
=======
	const crm = await wmill.getResource('f/sales/crm')
	return fetchCustomer(crm, customerId)
>>>>>>> fork/sales-crm
}

function fetchCustomer(resource: unknown, id: string, opts = { retries: 1 }) {
<<<<<<< HEAD
	return { resource, id, opts }
=======
	return { resource, id, opts, source: 'fork' }
>>>>>>> fork/sales-crm
}
`
