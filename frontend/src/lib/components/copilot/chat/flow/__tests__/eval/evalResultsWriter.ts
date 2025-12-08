// @ts-ignore
import { writeFile, mkdir } from 'fs/promises'
// @ts-ignore
import { join, dirname } from 'path'
// @ts-ignore
import { fileURLToPath } from 'url'
import type { EvalResult } from './evalRunner'

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

/**
 * Generates a timestamp string suitable for filenames.
 * Format: 2024-01-15T10-30-45-123Z (ISO but with dashes instead of colons)
 */
function generateTimestamp(): string {
	return new Date().toISOString().replace(/:/g, '-')
}

/**
 * Writes comparison results to files in the results folder.
 * Creates:
 * - {timestamp}.md - Summary with prompt and results table
 * - {timestamp}_{variant_name}.json - Flow JSON for each variant
 */
export async function writeComparisonResults(
	userPrompt: string,
	results: EvalResult[],
	outputDir?: string
): Promise<{ summaryPath: string; flowPaths: string[] }> {
	const resultsDir = outputDir ?? join(__dirname, 'results')
	const timestamp = generateTimestamp()

	// Ensure results directory exists
	await mkdir(resultsDir, { recursive: true })
	const resultFolder = join(resultsDir, timestamp)
	await mkdir(resultFolder, { recursive: true })

	// Build summary markdown
	const summaryLines: string[] = [
		`# Eval Results - ${timestamp}`,
		'',
		'## User Prompt',
		'```',
		userPrompt.trim(),
		'```',
		'',
		'## Results',
		'',
		'| Variant | Success | Total Tokens | Tool Calls | Iterations |',
		'|---------|---------|--------------|------------|------------|'
	]

	for (const result of results) {
		summaryLines.push(
			`| ${result.variantName} | ${result.success} | ${result.tokenUsage.total} | ${result.toolsCalled.length} | ${result.iterations} |`
		)
	}

	summaryLines.push('')
	summaryLines.push('## Flow Outputs')
	summaryLines.push('')

	const flowPaths: string[] = []

	for (const result of results) {
		const flowFilename = `${result.variantName}.json`
		const flowPath = join(resultFolder, flowFilename)
		flowPaths.push(flowPath)

		summaryLines.push(`- ${result.variantName}: ./${flowFilename}`)

		// Write flow JSON file
		const flowData = {
			variantName: result.variantName,
			success: result.success,
			error: result.error,
			modules: result.modules,
			toolsCalled: result.toolsCalled,
			toolCallDetails: result.toolCallDetails
		}
		await writeFile(flowPath, JSON.stringify(flowData, null, 2))
	}

	// Write summary markdown file
	const summaryPath = join(resultFolder, `summary.md`)
	await writeFile(summaryPath, summaryLines.join('\n'))

	return { summaryPath, flowPaths }
}
