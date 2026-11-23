import { readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'
import { z } from 'zod'
import { fenceSchema } from './displayCatalog'

// The model's guidance for charts and tables is generated from the catalog, so the schema it
// reads is the one its output is validated against. Regenerate with `npm run gen:display-catalog`.
const OUT = join(__dirname, '../../../../../system_prompts/base/display-catalog.gen.md')

const EXAMPLES: { title: string; value: unknown }[] = [
	{
		title: 'Bar chart, one series per y column',
		value: {
			chart: {
				type: 'bar',
				title: 'Runs per day',
				data: [
					{ day: '2026-11-10', success: 42, failure: 3 },
					{ day: '2026-11-11', success: 51, failure: 1 }
				],
				x: 'day',
				y: ['success', 'failure'],
				stacked: true
			}
		}
	},
	{
		title: 'Pie chart (exactly one y column)',
		value: {
			chart: {
				type: 'pie',
				data: [
					{ language: 'python3', scripts: 18 },
					{ language: 'bun', scripts: 25 }
				],
				x: 'language',
				y: 'scripts'
			}
		}
	},
	{
		title: 'A chart and a table together',
		value: {
			render_all: [
				{
					chart: {
						type: 'line',
						data: [
							{ hour: '10:00', p95_ms: 120 },
							{ hour: '11:00', p95_ms: 180 }
						],
						x: 'hour',
						y: 'p95_ms',
						y_label: 'ms'
					}
				},
				{
					'table-row-object': [
						{ hour: '10:00', p95_ms: 120, jobs: 310 },
						{ hour: '11:00', p95_ms: 180, jobs: 402 }
					]
				}
			]
		}
	}
]

function render(): string {
	const schema = z.toJSONSchema(fenceSchema, { unrepresentable: 'any' })
	const parts = [
		'### Chart and table schema',
		'',
		"A chart or table value, as JSON Schema. Beyond it: `x` and every `y` must be columns of `data`, `y` columns hold numbers, a scatter chart's `x` column holds numbers, and a pie chart takes exactly one `y`.",
		'',
		'```json',
		JSON.stringify(schema),
		'```',
		''
	]
	for (const { title, value } of EXAMPLES) {
		parts.push(`${title}:`, '', '```json', JSON.stringify(value, null, 2), '```', '')
	}
	return parts.join('\n')
}

describe('display catalog guidance', () => {
	it('examples are valid fence values', () => {
		for (const { title, value } of EXAMPLES) {
			expect(fenceSchema.safeParse(value).success, title).toBe(true)
		}
	})

	it('matches the catalog (npm run gen:display-catalog)', () => {
		const content = render()
		if (process.env.WM_GEN_DISPLAY_CATALOG) writeFileSync(OUT, content)
		expect(readFileSync(OUT, 'utf8')).toBe(content)
	})
})
