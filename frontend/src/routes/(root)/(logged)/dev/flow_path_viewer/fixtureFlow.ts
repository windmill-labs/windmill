import type { FlowModule, OpenFlow } from '$lib/gen'

function step(
	id: string,
	summary: string,
	language: 'python3' | 'bun' | 'deno' | 'bash' | 'postgresql',
	content: string,
	extra: Partial<FlowModule> = {}
): FlowModule {
	return {
		id,
		summary,
		value: { type: 'rawscript', input_transforms: {}, language, content },
		...extra
	}
}

const js = (expr: string) => ({ type: 'javascript' as const, expr })

/** Target of the fixture's subflow step, so the step panel has a nested graph to draw. */
export const subFixtureFlow: OpenFlow = {
	summary: 'Refund a line item (dev fixture subflow)',
	value: {
		modules: [
			step('a', 'Void the charge', 'bun', 'export async function main() {}\n'),
			step('b', 'Restock the item', 'python3', 'def main():\n    return "restocked"\n')
		]
	}
}

/**
 * Shape of the graph this fixture draws, so a change here can be judged against intent:
 * a straight step, a for-loop with two nested steps, a three-way branchone, a branchall
 * with a skip_failure branch, a subflow, a step carrying retry/cache/early-stop badges,
 * plus a failure module, a preprocessor, a note and a group.
 */
export const fixtureFlow = (subflowPath: string): OpenFlow => ({
	summary: 'Order fulfilment (dev fixture)',
	schema: {
		$schema: 'https://json-schema.org/draft/2020-12/schema',
		type: 'object',
		properties: {
			order_id: { type: 'string', description: 'Order to fulfil', default: 'ord_1234' },
			warehouse: {
				type: 'string',
				description: 'Warehouse to reserve stock from',
				enum: ['eu-west', 'us-east', 'ap-south'],
				default: 'eu-west'
			},
			max_items: { type: 'integer', description: 'Cap on line items', default: 25 },
			dry_run: { type: 'boolean', description: 'Skip every side effect', default: true }
		},
		required: ['order_id'],
		order: ['order_id', 'warehouse', 'max_items', 'dry_run']
	},
	value: {
		modules: [
			step('a', 'Fetch order', 'python3', 'def main(order_id: str):\n    return {"items": []}\n'),
			{
				id: 'b',
				summary: 'For each line item',
				value: {
					type: 'forloopflow',
					iterator: js('results.a.items'),
					skip_failures: false,
					parallel: true,
					parallelism: js('4'),
					modules: [
						step('c', 'Reserve stock', 'bun', 'export async function main() {}\n'),
						step('d', 'Write warehouse ledger', 'postgresql', 'INSERT INTO ledger VALUES ($1);\n')
					]
				}
			},
			{
				id: 'e',
				summary: 'Route by payment status',
				value: {
					type: 'branchone',
					branches: [
						{
							summary: 'Paid',
							expr: 'results.a.status === "paid"',
							modules: [step('f', 'Ship immediately', 'bun', 'export async function main() {}\n')]
						},
						{
							summary: 'Awaiting capture',
							expr: 'results.a.status === "pending"',
							modules: [step('g', 'Hold for capture', 'bun', 'export async function main() {}\n')]
						}
					],
					default: [step('h', 'Flag for review', 'bash', 'echo "manual review"\n')]
				}
			},
			{
				id: 'i',
				summary: 'Fan out notifications',
				value: {
					type: 'branchall',
					parallel: true,
					branches: [
						{
							summary: 'Customer email',
							modules: [step('j', 'Send email', 'bun', 'export async function main() {}\n')]
						},
						{
							summary: 'Internal Slack',
							skip_failure: true,
							modules: [step('k', 'Post to Slack', 'bun', 'export async function main() {}\n')]
						}
					]
				}
			},
			{
				id: 'm',
				summary: 'Refund rejected items',
				value: { type: 'flow', input_transforms: {}, path: subflowPath }
			},
			step('l', 'Close the order', 'python3', 'def main():\n    return "done"\n', {
				retry: { constant: { attempts: 3, seconds: 5 } },
				cache_ttl: 3600,
				stop_after_if: { expr: 'result === "done"', skip_if_stopped: true }
			})
		],
		failure_module: step(
			'failure',
			'Report the failure',
			'bun',
			'export async function main(error: any) {}\n'
		),
		preprocessor_module: step(
			'preprocessor',
			'Normalise the trigger payload',
			'bun',
			'export async function main(event: any) {\n\treturn event\n}\n'
		),
		notes: [
			{
				id: 'note_1',
				type: 'free',
				color: 'yellow',
				text: 'Stock reservation is idempotent — replaying the loop is safe.',
				position: { x: 420, y: 120 },
				size: { width: 220, height: 90 }
			}
		],
		groups: [
			{
				summary: 'Fulfilment',
				note: 'Everything between reserving stock and routing on payment.',
				start_id: 'b',
				end_id: 'e'
			}
		]
	}
})
