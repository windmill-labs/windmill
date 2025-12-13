// @ts-ignore
import { writeFile, mkdir } from 'fs/promises'
// @ts-ignore
import { join, dirname } from 'path'
// @ts-ignore
import { fileURLToPath } from 'url'
import type { BaseEvalResult } from './types'

/**
 * Generates a timestamp string suitable for filenames.
 * Format: 2024-01-15T10-30-45-123Z (ISO but with dashes instead of colons)
 */
export function generateTimestamp(): string {
	return new Date().toISOString().replace(/:/g, '-')
}

/**
 * Parameters for writing comparison results.
 */
export interface WriteResultsParams<TOutput> {
	/** User prompt that was tested */
	userPrompt: string
	/** Results from all variants */
	results: BaseEvalResult<TOutput>[]
	/** Directory to write results to */
	outputDir: string
	/** Function to format domain-specific output for JSON files */
	formatOutput: (output: TOutput) => unknown
	/** Label for the output type (e.g., 'flow', 'app') */
	outputLabel?: string
}

/**
 * Writes comparison results to files in the results folder.
 * Creates:
 * - summary.md - Summary with prompt and results table
 * - {variant_name}.json - Full result with metadata for each variant
 * - {variant_name}_{outputLabel}.json - Clean output for each variant
 */
export async function writeComparisonResults<TOutput>(
	params: WriteResultsParams<TOutput>
): Promise<{ summaryPath: string; outputPaths: string[] }> {
	const { userPrompt, results, outputDir, formatOutput, outputLabel = 'output' } = params
	const timestamp = generateTimestamp()

	// Ensure results directory exists
	await mkdir(outputDir, { recursive: true })
	const resultFolder = join(outputDir, timestamp)
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

	const outputPaths: string[] = []

	for (const result of results) {
		const resultFilename = `${result.variantName}.json`
		const resultPath = join(resultFolder, resultFilename)
		outputPaths.push(resultPath)

		const outputFilename = `${result.variantName}_${outputLabel}.json`
		const outputPath = join(resultFolder, outputFilename)

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

		// Write clean output JSON file (domain-specific format)
		const outputData = formatOutput(result.output)
		await writeFile(outputPath, JSON.stringify(outputData, null, 2))
	}

	// Write summary markdown file
	const summaryPath = join(resultFolder, `summary.md`)
	await writeFile(summaryPath, summaryLines.join('\n'))

	return { summaryPath, outputPaths }
}
