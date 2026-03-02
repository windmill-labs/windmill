import type { AppFiles, BackendRunnable, InlineScript } from '../../app/core'

/**
 * Backend runnable metadata stored in meta.json files.
 */
interface BackendMeta {
	name: string
	language: 'bun' | 'python3'
}

/**
 * Recursively reads all files in a directory and returns them as a record.
 * File paths are relative to the base directory with a leading '/'.
 */
async function readFilesRecursively(
	dir: string,
	basePath: string = ''
): Promise<Record<string, string>> {
	// @ts-ignore - Node.js fs/promises
	const { readdir, readFile } = await import('fs/promises')
	// @ts-ignore - Node.js path
	const { join } = await import('path')

	const result: Record<string, string> = {}
	const entries = await readdir(dir, { withFileTypes: true })

	for (const entry of entries) {
		const fullPath = join(dir, entry.name)
		const relativePath = basePath ? `${basePath}/${entry.name}` : `/${entry.name}`

		if (entry.isDirectory()) {
			const subFiles = await readFilesRecursively(fullPath, relativePath)
			Object.assign(result, subFiles)
		} else {
			const content = await readFile(fullPath, 'utf-8')
			result[relativePath] = content
		}
	}

	return result
}

/**
 * Loads frontend files from a directory.
 * All files are read recursively and paths become keys with leading '/'.
 */
async function loadFrontend(frontendPath: string): Promise<Record<string, string>> {
	// @ts-ignore - Node.js fs/promises
	const { access } = await import('fs/promises')

	try {
		await access(frontendPath)
	} catch {
		// Directory doesn't exist, return empty
		return {}
	}

	return readFilesRecursively(frontendPath)
}

/**
 * Loads backend runnables from a directory.
 * Each subdirectory is a runnable with:
 * - main.ts or main.py: The code content
 * - meta.json: Metadata { name, language }
 */
async function loadBackend(backendPath: string): Promise<Record<string, BackendRunnable>> {
	// @ts-ignore - Node.js fs/promises
	const { readdir, readFile, access } = await import('fs/promises')
	// @ts-ignore - Node.js path
	const { join } = await import('path')

	try {
		await access(backendPath)
	} catch {
		// Directory doesn't exist, return empty
		return {}
	}

	const result: Record<string, BackendRunnable> = {}
	const entries = await readdir(backendPath, { withFileTypes: true })

	for (const entry of entries) {
		if (!entry.isDirectory()) continue

		const runnableKey = entry.name
		const runnablePath = join(backendPath, entry.name)

		// Read meta.json
		const metaPath = join(runnablePath, 'meta.json')
		let meta: BackendMeta
		try {
			const metaContent = await readFile(metaPath, 'utf-8')
			meta = JSON.parse(metaContent)
		} catch {
			console.warn(`Missing or invalid meta.json for runnable '${runnableKey}', skipping`)
			continue
		}

		// Find and read the main file (main.ts or main.py)
		const runnableFiles = await readdir(runnablePath)
		const mainFile = runnableFiles.find((f) => f === 'main.ts' || f === 'main.py')

		if (!mainFile) {
			console.warn(`No main.ts or main.py found for runnable '${runnableKey}', skipping`)
			continue
		}

		const content = await readFile(join(runnablePath, mainFile), 'utf-8')

		const inlineScript: InlineScript = {
			language: meta.language,
			content
		}

		result[runnableKey] = {
			name: meta.name,
			type: 'inline',
			inlineScript
		}
	}

	return result
}

/**
 * Loads an app fixture from a directory structure.
 *
 * Expected structure:
 * ```
 * fixturePath/
 * ├── frontend/
 * │   └── index.tsx           # → frontend["/index.tsx"]
 * │   └── components/
 * │       └── Button.tsx      # → frontend["/components/Button.tsx"]
 * └── backend/
 *     └── incrementCounter/
 *         ├── main.ts         # The code content
 *         └── meta.json       # { "name": "...", "language": "bun" }
 * ```
 *
 * @param fixturePath - Path to the fixture directory
 * @returns AppFiles object with frontend and backend
 */
export async function loadAppFixture(fixturePath: string): Promise<AppFiles> {
	// @ts-ignore - Node.js path
	const { join } = await import('path')

	const frontend = await loadFrontend(join(fixturePath, 'frontend'))
	const backend = await loadBackend(join(fixturePath, 'backend'))

	return { frontend, backend }
}

/**
 * Loads an app fixture and returns the separate frontend and backend objects.
 * Convenience function for use with runAppEval options.
 */
export async function loadAppFixtureForEval(
	fixturePath: string
): Promise<{
	initialFrontend: Record<string, string>
	initialBackend: Record<string, BackendRunnable>
}> {
	const { frontend, backend } = await loadAppFixture(fixturePath)
	return {
		initialFrontend: frontend,
		initialBackend: backend
	}
}
