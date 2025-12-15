import type { AppFiles, BackendRunnable } from '../../app/core'
import type { AppEvalResult } from './appEvalRunner'
import { generateTimestamp } from '../shared'

/**
 * Writes frontend files to a folder, preserving directory structure.
 * File paths like "/components/Button.tsx" become "frontend/components/Button.tsx"
 */
async function writeFrontendFiles(
	frontend: Record<string, string>,
	frontendPath: string
): Promise<void> {
	// @ts-ignore - Node.js fs/promises
	const { writeFile, mkdir } = await import('fs/promises')
	// @ts-ignore - Node.js path
	const { join, dirname } = await import('path')

	for (const [filePath, content] of Object.entries(frontend)) {
		// Remove leading slash and join with frontend path
		const relativePath = filePath.startsWith('/') ? filePath.slice(1) : filePath
		const fullPath = join(frontendPath, relativePath)

		// Ensure parent directory exists
		await mkdir(dirname(fullPath), { recursive: true })

		await writeFile(fullPath, content)
	}
}

/**
 * Writes backend runnables to a folder structure.
 * Each runnable becomes a folder with main.ts/main.py and meta.json
 */
async function writeBackendRunnables(
	backend: Record<string, BackendRunnable>,
	backendPath: string
): Promise<void> {
	// @ts-ignore - Node.js fs/promises
	const { writeFile, mkdir } = await import('fs/promises')
	// @ts-ignore - Node.js path
	const { join } = await import('path')

	for (const [key, runnable] of Object.entries(backend)) {
		const runnablePath = join(backendPath, key)
		await mkdir(runnablePath, { recursive: true })

		// Write meta.json
		const meta: { name: string; language?: string; type?: string; path?: string } = {
			name: runnable.name
		}

		if (runnable.type === 'inline' && runnable.inlineScript) {
			meta.language = runnable.inlineScript.language

			// Write main file
			const extension = runnable.inlineScript.language === 'python3' ? 'py' : 'ts'
			const mainPath = join(runnablePath, `main.${extension}`)
			await writeFile(mainPath, runnable.inlineScript.content)
		} else {
			// For non-inline runnables, store type and path in meta
			meta.type = runnable.type
			if (runnable.path) {
				meta.path = runnable.path
			}
		}

		const metaPath = join(runnablePath, 'meta.json')
		await writeFile(metaPath, JSON.stringify(meta, null, '\t'))
	}
}

/**
 * Writes app files (frontend + backend) to a folder structure.
 */
async function writeAppToFolder(appFiles: AppFiles, folderPath: string): Promise<void> {
	// @ts-ignore - Node.js path
	const { join } = await import('path')

	if (Object.keys(appFiles.frontend).length > 0) {
		await writeFrontendFiles(appFiles.frontend, join(folderPath, 'frontend'))
	}

	if (Object.keys(appFiles.backend).length > 0) {
		await writeBackendRunnables(appFiles.backend, join(folderPath, 'backend'))
	}
}

/**
 * Parameters for writing app comparison results.
 */
export interface WriteAppResultsParams {
	userPrompt: string
	results: AppEvalResult[]
	outputDir: string
}

/**
 * Writes app comparison results to a folder-based structure.
 *
 * Creates:
 * ```
 * results/{timestamp}/
 * ├── summary.md
 * └── {variant_name}/
 *     ├── details.json    # Metadata (toolsCalled, evaluationResult, etc.)
 *     ├── frontend/       # Frontend files
 *     │   └── index.tsx
 *     └── backend/        # Backend runnables
 *         └── myFunction/
 *             ├── main.ts
 *             └── meta.json
 * ```
 */
export async function writeAppComparisonResultsToFolders(
	params: WriteAppResultsParams
): Promise<{ summaryPath: string; variantPaths: string[] }> {
	// @ts-ignore - Node.js fs/promises
	const { writeFile, mkdir } = await import('fs/promises')
	// @ts-ignore - Node.js path
	const { join } = await import('path')

	const { userPrompt, results, outputDir } = params
	const timestamp = generateTimestamp()

	// Ensure results directory exists
	await mkdir(outputDir, { recursive: true })
	const resultFolder = join(outputDir, timestamp)
	await mkdir(resultFolder, { recursive: true })

	// Check if any results have evaluation data
	const hasEvaluation = results.some((r) => r.evaluationResult)

	// Build summary markdown
	const summaryLines: string[] = [
		`# App Eval Results - ${timestamp}`,
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

	const variantPaths: string[] = []

	// Write each variant to its own folder
	for (const result of results) {
		const variantFolder = join(resultFolder, result.variantName)
		await mkdir(variantFolder, { recursive: true })
		variantPaths.push(variantFolder)

		// Write details.json (metadata without app files)
		const details = {
			variantName: result.variantName,
			success: result.success,
			error: result.error ?? null,
			evaluationResult: result.evaluationResult ?? null,
			toolsCalled: result.toolsCalled,
			toolCallDetails: result.toolCallDetails,
			tokenUsage: result.tokenUsage,
			iterations: result.iterations
		}
		await writeFile(join(variantFolder, 'details.json'), JSON.stringify(details, null, '\t'))

		// Write app files to frontend/ and backend/ folders
		await writeAppToFolder(result.files, variantFolder)
	}

	// Write summary markdown file
	const summaryPath = join(resultFolder, 'summary.md')
	await writeFile(summaryPath, summaryLines.join('\n'))

	return { summaryPath, variantPaths }
}
