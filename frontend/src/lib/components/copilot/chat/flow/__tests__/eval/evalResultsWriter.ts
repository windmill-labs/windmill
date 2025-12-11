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

	// Check if any results have evaluation data
	const hasEvaluation = results.some((r) => r.evaluationResult)

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
		''
	]

	// Add results table header based on whether evaluation data exists
	if (hasEvaluation) {
		summaryLines.push(
			'| Variant | Success | Total Tokens | Tool Calls | Iterations | Resemblance Score |'
		)
		summaryLines.push(
			'|---------|---------|--------------|------------|------------|-------------------|'
		)
	} else {
		summaryLines.push('| Variant | Success | Total Tokens | Tool Calls | Iterations |')
		summaryLines.push('|---------|---------|--------------|------------|------------|')
	}

	for (const result of results) {
		const baseRow = `| ${result.variantName} | ${result.success} | ${result.tokenUsage.total} | ${result.toolsCalled.length} | ${result.iterations}`
		if (hasEvaluation) {
			const score = result.evaluationResult?.resemblanceScore ?? 'N/A'
			summaryLines.push(`${baseRow} | ${score} |`)
		} else {
			summaryLines.push(`${baseRow} |`)
		}
	}

	// Add evaluation details section if available
	if (hasEvaluation) {
		summaryLines.push('')
		summaryLines.push('## Evaluation Details')
		summaryLines.push('')
		for (const result of results) {
			if (result.evaluationResult) {
				summaryLines.push(`### ${result.variantName}`)
				summaryLines.push('')
				summaryLines.push(`**Score:** ${result.evaluationResult.resemblanceScore}/100`)
				summaryLines.push('')
				summaryLines.push(`**Statement:** ${result.evaluationResult.statement}`)
				summaryLines.push('')
				if (
					result.evaluationResult.missingRequirements &&
					result.evaluationResult.missingRequirements.length > 0
				) {
					summaryLines.push('**Missing Requirements:**')
					for (const req of result.evaluationResult.missingRequirements) {
						summaryLines.push(`- ${req}`)
					}
					summaryLines.push('')
				}
				if (result.evaluationResult.error) {
					summaryLines.push(`**Error:** ${result.evaluationResult.error}`)
					summaryLines.push('')
				}
			}
		}
	}

	// Add errors section for failed variants
	const failedResults = results.filter((r) => !r.success && r.error)
	if (failedResults.length > 0) {
		summaryLines.push('')
		summaryLines.push('## Errors')
		summaryLines.push('')
		for (const result of failedResults) {
			summaryLines.push(`### ${result.variantName}`)
			summaryLines.push('')
			summaryLines.push('```')
			summaryLines.push(result.error!)
			summaryLines.push('```')
			summaryLines.push('')
		}
	}

	const flowPaths: string[] = []

	for (const result of results) {
		const resultFilename = `${result.variantName}.json`
		const resultPath = join(resultFolder, resultFilename)
		flowPaths.push(resultPath)

		const flowFilename = `${result.variantName}_flow.json`
		const flowPath = join(resultFolder, flowFilename)

		// Write result JSON file (with metadata)
		const resultData = {
			variantName: result.variantName,
			success: result.success,
			error: result.error,
			evaluationResult: result.evaluationResult,
			toolsCalled: result.toolsCalled,
			toolCallDetails: result.toolCallDetails,
			messages: result.messages
		}
		await writeFile(resultPath, JSON.stringify(resultData, null, 2))

		// Write flow definition JSON file (clean flow format)
		const flowData = {
			summary: result.flow.summary ?? '',
			value: {
				modules: result.flow.value.modules
			},
			schema: result.flow.schema ?? {
				$schema: 'https://json-schema.org/draft/2020-12/schema',
				properties: {},
				required: [],
				type: 'object'
			}
		}
		await writeFile(flowPath, JSON.stringify(flowData, null, 2))
	}

	// Write summary markdown file
	const summaryPath = join(resultFolder, `summary.md`)
	await writeFile(summaryPath, summaryLines.join('\n'))

	return { summaryPath, flowPaths }
}
