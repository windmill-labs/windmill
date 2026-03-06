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

const TS_AWAIT_RE = /(?:(?:const|let|var)\s+(\w+)\s*=\s*)?await\s+(\w+)\s*\(([^)]*)\)/

function matchTsTaskCall(line: string, taskMap: Map<string, WacTask>): WacStep | undefined {
	const m = line.match(TS_AWAIT_RE)
	if (!m) return undefined
	const task = taskMap.get(m[2])
	if (!task) return undefined
	return { type: 'step', name: task.name, script: task.script, varName: m[1] }
}

function collectBraceBlock(lines: string[], start: number): { bodyLines: string[]; end: number } {
	const bodyLines: string[] = []
	let depth = 1
	let j = start
	while (j < lines.length && depth > 0) {
		if (lines[j].includes('{')) depth++
		if (lines[j].includes('}')) depth--
		if (depth > 0) bodyLines.push(lines[j].trim())
		j++
	}
	return { bodyLines, end: j - 1 }
}

function extractTaskSteps(bodyLines: string[], taskMap: Map<string, WacTask>): WacStep[] {
	const steps: WacStep[] = []
	for (const line of bodyLines) {
		const step = matchTsTaskCall(line, taskMap)
		if (step) steps.push(step)
	}
	return steps
}

function parseBodyStatements(body: string, tasks: WacTask[], out: WacStatement[]) {
	const taskMap = new Map(tasks.map((t) => [t.name, t]))
	const lines = body.split('\n')

	for (let i = 0; i < lines.length; i++) {
		const line = lines[i].trim()

		// await taskName(...)
		const step = matchTsTaskCall(line, taskMap)
		if (step) {
			out.push(step)
			continue
		}

		// Promise.all([taskA(...), taskB(...)])
		if (/await\s+Promise\.all\s*\(\s*\[/.test(line)) {
			let block = line
			let j = i
			while (!block.includes(']') && j < lines.length - 1) {
				j++
				block += '\n' + lines[j]
			}
			i = j

			const steps: WacStep[] = []
			const callRegex = /(\w+)\s*\(/g
			let cm
			while ((cm = callRegex.exec(block)) !== null) {
				const task = taskMap.get(cm[1])
				if (task) steps.push({ type: 'step', name: task.name, script: task.script })
			}
			if (steps.length > 0) out.push({ type: 'parallel', steps })
			continue
		}

		// if (...) { ... }
		const ifMatch = line.match(/^if\s*\((.+?)\)\s*\{/)
		if (ifMatch) {
			const { bodyLines, end } = collectBraceBlock(lines, i + 1)
			i = end
			out.push({
				type: 'branch',
				condition: ifMatch[1],
				body: extractTaskSteps(bodyLines, taskMap)
			})
			continue
		}

		// for (const x of items) { ... }
		const forMatch = line.match(/^for\s*\(\s*(?:const|let|var)\s+(\w+)\s+of\s+(\w+)\s*\)\s*\{/)
		if (forMatch) {
			const { bodyLines, end } = collectBraceBlock(lines, i + 1)
			i = end
			out.push({
				type: 'loop',
				iterator: forMatch[2],
				variable: forMatch[1],
				body: extractTaskSteps(bodyLines, taskMap)
			})
			continue
		}
	}
}

const PY_AWAIT_RE = /(?:(\w+)\s*=\s*)?await\s+(\w+)\s*\(/

function matchPyTaskCall(line: string, taskMap: Map<string, WacTask>): WacStep | undefined {
	const m = line.match(PY_AWAIT_RE)
	if (!m) return undefined
	const task = taskMap.get(m[2])
	if (!task) return undefined
	return { type: 'step', name: task.name, script: task.script, varName: m[1] }
}

function collectIndentBlock(
	lines: string[],
	start: number,
	parentIndent: number
): { bodyLines: string[]; end: number } {
	const bodyLines: string[] = []
	let j = start
	while (j < lines.length) {
		const nextLine = lines[j]
		const nextIndent = nextLine.search(/\S/)
		if (nextLine.trim() === '' || nextIndent > parentIndent) {
			bodyLines.push(nextLine.trim())
			j++
		} else {
			break
		}
	}
	return { bodyLines, end: j - 1 }
}

function extractPyTaskSteps(bodyLines: string[], taskMap: Map<string, WacTask>): WacStep[] {
	const steps: WacStep[] = []
	for (const line of bodyLines) {
		const step = matchPyTaskCall(line, taskMap)
		if (step) steps.push(step)
	}
	return steps
}

function parsePythonBody(body: string, tasks: WacTask[], out: WacStatement[]) {
	const taskMap = new Map(tasks.map((t) => [t.name, t]))
	const lines = body.split('\n')

	for (let i = 0; i < lines.length; i++) {
		const line = lines[i]
		const trimmed = line.trim()

		// result = await task_name(...)
		const step = matchPyTaskCall(trimmed, taskMap)
		if (step) {
			out.push(step)
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
				const task = taskMap.get(cm[1])
				if (task) steps.push({ type: 'step', name: task.name, script: task.script })
			}
			if (steps.length > 0) out.push({ type: 'parallel', steps })
			continue
		}

		// if condition:
		const ifMatch = trimmed.match(/^if\s+(.+?)\s*:/)
		if (ifMatch) {
			const indent = line.search(/\S/)
			const { bodyLines, end } = collectIndentBlock(lines, i + 1, indent)
			i = end
			out.push({
				type: 'branch',
				condition: ifMatch[1],
				body: extractPyTaskSteps(bodyLines, taskMap)
			})
			continue
		}

		// for variable in iterator:
		const forMatch = trimmed.match(/^for\s+(\w+)\s+in\s+(\w+)\s*:/)
		if (forMatch) {
			const indent = line.search(/\S/)
			const { bodyLines, end } = collectIndentBlock(lines, i + 1, indent)
			i = end
			out.push({
				type: 'loop',
				iterator: forMatch[2],
				variable: forMatch[1],
				body: extractPyTaskSteps(bodyLines, taskMap)
			})
			continue
		}
	}
}

function statementsToModules(statements: WacStatement[]): FlowModule[] {
	const counter = { n: 0 }
	return statements.map((s) => statementToModule(s, counter))
}

function statementToModule(stmt: WacStatement, counter: { n: number }): FlowModule {
	const id = stmt.type === 'step' ? stmt.name : `${stmt.type}_${counter.n++}`

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
					modules: [statementToModule(s, counter)]
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
							modules: stmt.body.map((s) => statementToModule(s, counter))
						}
					],
					default: []
				}
			}
		}
		case 'loop': {
			const value: ForloopFlow = {
				type: 'forloopflow',
				modules: stmt.body.map((s) => statementToModule(s, counter)),
				iterator: { type: 'javascript', expr: stmt.iterator },
				skip_failures: false
			}
			return { id, summary: `for ${stmt.variable} of ${stmt.iterator}`, value }
		}
	}
}
