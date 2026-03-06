/**
 * Lightweight parser that detects workflow-as-code patterns in TypeScript/Python code
 * and converts them to FlowModule[] for rendering in FlowGraphV2.
 *
 * This is a v0 regex-based parser. It will be replaced by the WASM parser
 * (windmill-parser-wasm-wac) once that package is built and published.
 */
import type { FlowModule, BranchAll, ForloopFlow } from '$lib/gen'

interface WacTask {
	name: string
	script: string
}

interface WacStep {
	type: 'step'
	name: string
	script: string
	varName?: string
}

interface WacParallel {
	type: 'parallel'
	steps: WacStep[]
}

interface WacBranch {
	type: 'branch'
	condition: string
	body: WacStatement[]
}

interface WacLoop {
	type: 'loop'
	iterator: string
	variable: string
	body: WacStatement[]
}

type WacStatement = WacStep | WacParallel | WacBranch | WacLoop

export interface WacParseResult {
	modules: FlowModule[]
	tasks: WacTask[]
	error?: string
}

export function isWorkflowAsCode(code: string, language: string): boolean {
	if (language === 'python3') {
		return /^\s*@workflow\s*$/m.test(code) || /from\s+wmill\s+import.*workflow/.test(code)
	}
	if (language === 'bun' || language === 'deno') {
		return (
			/workflow\s*\(/.test(code) &&
			/task\s*\(/.test(code) &&
			/import.*(?:workflow|task).*from\s+['"]windmill-client['"]/.test(code)
		)
	}
	return false
}

export function parseWacCode(code: string, language: string): WacParseResult {
	try {
		if (language === 'python3') {
			return parsePythonWac(code)
		}
		if (language === 'bun' || language === 'deno') {
			return parseTsWac(code)
		}
		return { modules: [], tasks: [], error: `Unsupported language: ${language}` }
	} catch (e) {
		return { modules: [], tasks: [], error: (e as Error).message }
	}
}

function parseTsWac(code: string): WacParseResult {
	const tasks: WacTask[] = []
	const statements: WacStatement[] = []

	// Detect task declarations: const name = task(async function name(...) { ... })
	// or const name = task("path", async function name(...) { ... })
	const taskRegex =
		/const\s+(\w+)\s*=\s*task\s*\(\s*(?:"([^"]+)"\s*,\s*)?async\s+(?:function\s+\w*\s*)?\(/g
	let m
	while ((m = taskRegex.exec(code)) !== null) {
		tasks.push({ name: m[1], script: m[2] || m[1] })
	}

	// Find the workflow body
	const workflowMatch = code.match(
		/export\s+default\s+workflow\s*\(\s*async\s*(?:\([^)]*\)|\w+)\s*(?::\s*\w+)?\s*=>\s*\{([\s\S]*)\}\s*\)\s*;?\s*$/
	)
	if (!workflowMatch) {
		return { modules: [], tasks, error: 'No workflow() wrapper found' }
	}
	const body = workflowMatch[1]
	parseBodyStatements(body, tasks, statements)

	return { modules: statementsToModules(statements), tasks }
}

function parsePythonWac(code: string): WacParseResult {
	const tasks: WacTask[] = []
	const statements: WacStatement[] = []

	// Detect @task decorated functions
	const taskRegex = /^@task(?:\(path="([^"]+)"\))?\s*\nasync\s+def\s+(\w+)/gm
	let m
	while ((m = taskRegex.exec(code)) !== null) {
		tasks.push({ name: m[2], script: m[1] || m[2] })
	}

	// Find the @workflow function body
	const workflowMatch = code.match(/@workflow\s*\nasync\s+def\s+\w+\s*\([^)]*\)\s*:\s*\n([\s\S]+)$/)
	if (!workflowMatch) {
		return { modules: [], tasks, error: 'No @workflow function found' }
	}
	const body = workflowMatch[1]
	parsePythonBody(body, tasks, statements)

	return { modules: statementsToModules(statements), tasks }
}

function parseBodyStatements(body: string, tasks: WacTask[], out: WacStatement[]) {
	const taskNames = new Set(tasks.map((t) => t.name))
	const lines = body.split('\n')

	for (let i = 0; i < lines.length; i++) {
		const line = lines[i].trim()

		// await taskName(...)
		const awaitMatch = line.match(
			/(?:(?:const|let|var)\s+(\w+)\s*=\s*)?await\s+(\w+)\s*\(([^)]*)\)/
		)
		if (awaitMatch && taskNames.has(awaitMatch[2])) {
			const task = tasks.find((t) => t.name === awaitMatch[2])!
			out.push({
				type: 'step',
				name: task.name,
				script: task.script,
				varName: awaitMatch[1]
			})
			continue
		}

		// Promise.all([taskA(...), taskB(...)])
		const promiseAllMatch = line.match(/await\s+Promise\.all\s*\(\s*\[/)
		if (promiseAllMatch) {
			const steps: WacStep[] = []
			// Collect lines until closing ])
			let block = line
			let j = i
			while (!block.includes(']') && j < lines.length - 1) {
				j++
				block += '\n' + lines[j]
			}
			i = j

			const callRegex = /(\w+)\s*\(/g
			let cm
			while ((cm = callRegex.exec(block)) !== null) {
				if (taskNames.has(cm[1])) {
					const task = tasks.find((t) => t.name === cm[1])!
					steps.push({ type: 'step', name: task.name, script: task.script })
				}
			}
			if (steps.length > 0) {
				out.push({ type: 'parallel', steps })
			}
			continue
		}

		// if (...) { ... }
		const ifMatch = line.match(/^if\s*\((.+?)\)\s*\{/)
		if (ifMatch) {
			const condition = ifMatch[1]
			const branchBody: WacStatement[] = []
			let depth = 1
			let j = i + 1
			while (j < lines.length && depth > 0) {
				if (lines[j].includes('{')) depth++
				if (lines[j].includes('}')) depth--
				if (depth > 0) {
					const bl = lines[j].trim()
					const bAwait = bl.match(
						/(?:(?:const|let|var)\s+(\w+)\s*=\s*)?await\s+(\w+)\s*\(([^)]*)\)/
					)
					if (bAwait && taskNames.has(bAwait[2])) {
						const task = tasks.find((t) => t.name === bAwait[2])!
						branchBody.push({
							type: 'step',
							name: task.name,
							script: task.script,
							varName: bAwait[1]
						})
					}
				}
				j++
			}
			i = j - 1
			out.push({ type: 'branch', condition, body: branchBody })
			continue
		}

		// for (const x of items) { ... }
		const forMatch = line.match(/^for\s*\(\s*(?:const|let|var)\s+(\w+)\s+of\s+(\w+)\s*\)\s*\{/)
		if (forMatch) {
			const variable = forMatch[1]
			const iterator = forMatch[2]
			const loopBody: WacStatement[] = []
			let depth = 1
			let j = i + 1
			while (j < lines.length && depth > 0) {
				if (lines[j].includes('{')) depth++
				if (lines[j].includes('}')) depth--
				if (depth > 0) {
					const bl = lines[j].trim()
					const bAwait = bl.match(
						/(?:(?:const|let|var)\s+(\w+)\s*=\s*)?await\s+(\w+)\s*\(([^)]*)\)/
					)
					if (bAwait && taskNames.has(bAwait[2])) {
						const task = tasks.find((t) => t.name === bAwait[2])!
						loopBody.push({
							type: 'step',
							name: task.name,
							script: task.script,
							varName: bAwait[1]
						})
					}
				}
				j++
			}
			i = j - 1
			out.push({ type: 'loop', iterator, variable, body: loopBody })
			continue
		}
	}
}

function parsePythonBody(body: string, tasks: WacTask[], out: WacStatement[]) {
	const taskNames = new Set(tasks.map((t) => t.name))
	const lines = body.split('\n')

	for (let i = 0; i < lines.length; i++) {
		const line = lines[i]
		const trimmed = line.trim()

		// result = await task_name(...)
		const awaitMatch = trimmed.match(/(?:(\w+)\s*=\s*)?await\s+(\w+)\s*\(/)
		if (awaitMatch && taskNames.has(awaitMatch[2])) {
			const task = tasks.find((t) => t.name === awaitMatch[2])!
			out.push({
				type: 'step',
				name: task.name,
				script: task.script,
				varName: awaitMatch[1]
			})
			continue
		}

		// asyncio.gather(...)
		if (trimmed.includes('asyncio.gather(')) {
			const steps: WacStep[] = []
			let block = trimmed
			let j = i
			while (!block.includes(')') && j < lines.length - 1) {
				j++
				block += '\n' + lines[j]
			}
			i = j
			const callRegex = /(\w+)\s*\(/g
			let cm
			while ((cm = callRegex.exec(block)) !== null) {
				if (taskNames.has(cm[1])) {
					const task = tasks.find((t) => t.name === cm[1])!
					steps.push({ type: 'step', name: task.name, script: task.script })
				}
			}
			if (steps.length > 0) {
				out.push({ type: 'parallel', steps })
			}
			continue
		}

		// if condition:
		const ifMatch = trimmed.match(/^if\s+(.+?)\s*:/)
		if (ifMatch) {
			const condition = ifMatch[1]
			const indent = line.search(/\S/)
			const branchBody: WacStatement[] = []
			let j = i + 1
			while (j < lines.length) {
				const nextLine = lines[j]
				const nextIndent = nextLine.search(/\S/)
				if (nextLine.trim() === '' || nextIndent > indent) {
					const bl = nextLine.trim()
					const bAwait = bl.match(/(?:(\w+)\s*=\s*)?await\s+(\w+)\s*\(/)
					if (bAwait && taskNames.has(bAwait[2])) {
						const task = tasks.find((t) => t.name === bAwait[2])!
						branchBody.push({
							type: 'step',
							name: task.name,
							script: task.script,
							varName: bAwait[1]
						})
					}
					j++
				} else {
					break
				}
			}
			i = j - 1
			out.push({ type: 'branch', condition, body: branchBody })
			continue
		}

		// for variable in iterator:
		const forMatch = trimmed.match(/^for\s+(\w+)\s+in\s+(\w+)\s*:/)
		if (forMatch) {
			const variable = forMatch[1]
			const iterator = forMatch[2]
			const indent = line.search(/\S/)
			const loopBody: WacStatement[] = []
			let j = i + 1
			while (j < lines.length) {
				const nextLine = lines[j]
				const nextIndent = nextLine.search(/\S/)
				if (nextLine.trim() === '' || nextIndent > indent) {
					const bl = nextLine.trim()
					const bAwait = bl.match(/(?:(\w+)\s*=\s*)?await\s+(\w+)\s*\(/)
					if (bAwait && taskNames.has(bAwait[2])) {
						const task = tasks.find((t) => t.name === bAwait[2])!
						loopBody.push({
							type: 'step',
							name: task.name,
							script: task.script,
							varName: bAwait[1]
						})
					}
					j++
				} else {
					break
				}
			}
			i = j - 1
			out.push({ type: 'loop', iterator, variable, body: loopBody })
			continue
		}
	}
}

let moduleCounter = 0

function statementsToModules(statements: WacStatement[]): FlowModule[] {
	moduleCounter = 0
	return statements.map(statementToModule)
}

function statementToModule(stmt: WacStatement): FlowModule {
	const id = stmt.type === 'step' ? stmt.name : `${stmt.type}_${moduleCounter++}`

	switch (stmt.type) {
		case 'step':
			return {
				id: stmt.varName || stmt.name,
				summary: stmt.name,
				value: {
					type: 'script',
					path: stmt.script,
					input_transforms: {}
				}
			}
		case 'parallel': {
			const value: BranchAll = {
				type: 'branchall',
				branches: stmt.steps.map((s) => ({
					summary: s.name,
					modules: [statementToModule(s)]
				})),
				parallel: true
			}
			return { id, value }
		}
		case 'branch': {
			return {
				id,
				value: {
					type: 'branchone',
					branches: [
						{
							summary: stmt.condition,
							expr: stmt.condition,
							modules: stmt.body.map(statementToModule)
						}
					],
					default: []
				}
			}
		}
		case 'loop': {
			const value: ForloopFlow = {
				type: 'forloopflow',
				modules: stmt.body.map(statementToModule),
				iterator: { type: 'javascript', expr: stmt.iterator },
				skip_failures: false
			}
			return { id, summary: `for ${stmt.variable} of ${stmt.iterator}`, value }
		}
	}
}
