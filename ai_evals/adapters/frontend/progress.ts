export type FrontendBenchmarkProgressSurface = 'flow' | 'app' | 'script'

export type FrontendBenchmarkProgressEvent =
	| {
			type: 'run-start'
			surface: FrontendBenchmarkProgressSurface
			totalCases: number
			runs: number
			concurrency: number
	  }
	| {
			type: 'attempt-start'
			surface: FrontendBenchmarkProgressSurface
			caseId: string
			caseNumber: number
			totalCases: number
			attempt: number
			runs: number
	  }
	| {
			type: 'attempt-finish'
			surface: FrontendBenchmarkProgressSurface
			caseId: string
			caseNumber: number
			totalCases: number
			attempt: number
			runs: number
			passed: boolean
			durationMs: number
			judgeScore: number | null
			error: string | null
	  }
	| {
			type: 'assistant-message-start'
			surface: FrontendBenchmarkProgressSurface
			caseId: string
			caseNumber: number
			totalCases: number
			attempt: number
			runs: number
	  }
	| {
			type: 'assistant-chunk'
			surface: FrontendBenchmarkProgressSurface
			caseId: string
			caseNumber: number
			totalCases: number
			attempt: number
			runs: number
			chunk: string
	  }
	| {
			type: 'assistant-message-end'
			surface: FrontendBenchmarkProgressSurface
			caseId: string
			caseNumber: number
			totalCases: number
			attempt: number
			runs: number
	  }
	| {
			type: 'tool-call'
			surface: FrontendBenchmarkProgressSurface
			caseId: string
			caseNumber: number
			totalCases: number
			attempt: number
			runs: number
			toolName: string
			argumentsText: string
	  }

export const FRONTEND_BENCHMARK_PROGRESS_PREFIX = 'WMILL_FRONTEND_AI_EVAL_PROGRESS '

export function emitFrontendBenchmarkProgress(event: FrontendBenchmarkProgressEvent): void {
	process.stderr.write(
		`${FRONTEND_BENCHMARK_PROGRESS_PREFIX}${JSON.stringify(event)}\n`
	)
}

export function parseFrontendBenchmarkProgressLine(
	line: string
): FrontendBenchmarkProgressEvent | null {
	if (!line.startsWith(FRONTEND_BENCHMARK_PROGRESS_PREFIX)) {
		return null
	}

	try {
		const parsed = JSON.parse(
			line.slice(FRONTEND_BENCHMARK_PROGRESS_PREFIX.length)
		) as FrontendBenchmarkProgressEvent
		return parsed?.type ? parsed : null
	} catch {
		return null
	}
}

export function formatFrontendBenchmarkProgressEvent(
	event: FrontendBenchmarkProgressEvent
): string {
	switch (event.type) {
		case 'run-start':
			return `Running ${event.surface}: ${event.totalCases} cases x ${event.runs} run${event.runs === 1 ? '' : 's'}, concurrency ${event.concurrency}`
		case 'attempt-start':
			return `${formatCasePrefix(event.caseNumber, event.totalCases)} ${event.caseId} attempt ${event.attempt}/${event.runs}...`
		case 'attempt-finish': {
			const parts = [
				`${formatCasePrefix(event.caseNumber, event.totalCases)} ${event.caseId} attempt ${event.attempt}/${event.runs} ${event.passed ? 'pass' : 'fail'}`,
				formatDuration(event.durationMs)
			]
			if (event.judgeScore !== null) {
				parts.push(`judge ${formatNumber(event.judgeScore)}`)
			}
			if (event.error) {
				parts.push(truncateSingleLine(event.error, 120))
			}
			return parts.join(' | ')
		}
		case 'assistant-message-start':
		case 'assistant-chunk':
		case 'assistant-message-end':
			return ''
		case 'tool-call':
			return `${formatCasePrefix(event.caseNumber, event.totalCases)} ${event.caseId} attempt ${event.attempt}/${event.runs} tool ${event.toolName} ${truncateSingleLine(event.argumentsText, 200)}`
	}
}

function formatCasePrefix(caseNumber: number, totalCases: number): string {
	return `[${caseNumber}/${totalCases}]`
}

function formatDuration(durationMs: number): string {
	return `${formatNumber(durationMs / 1000)}s`
}

function formatNumber(value: number): string {
	return Number.isInteger(value) ? String(value) : value.toFixed(1)
}

function truncateSingleLine(value: string, maxLength: number): string {
	const normalized = value.replace(/\s+/g, ' ').trim()
	if (normalized.length <= maxLength) {
		return normalized
	}
	return `${normalized.slice(0, Math.max(0, maxLength - 3))}...`
}
