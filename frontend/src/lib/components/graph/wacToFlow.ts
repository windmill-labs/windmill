/**
 * Detect whether a script is a workflow-as-code entry point.
 */
export function isWorkflowAsCode(code: string, language: string): boolean {
	if (language === 'python3') {
		return /^\s*@workflow\s*$/m.test(code) || /from\s+wmill\s+import.*workflow/.test(code)
	}
	if (language === 'bun' || language === 'deno') {
		return (
			/workflow\s*\(/.test(code) &&
			/task\s*\(/.test(code) &&
			/import.*(?:workflow|task).*from\s+['"]windmill-client(?:@[^'"]*)?['"]/.test(code)
		)
	}
	return false
}
