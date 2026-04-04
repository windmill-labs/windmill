import { mkdir, rm, writeFile } from 'fs/promises'
import { dirname, join } from 'path'
import type {
	AppAIChatHelpers,
	AppFiles,
	BackendRunnable,
	DataTableSchema,
	LintResult,
	SelectedContext
} from '../../../../../frontend/src/lib/components/copilot/chat/app/core'

function createEmptyLintResult(): LintResult {
	return {
		errorCount: 0,
		warningCount: 0,
		errors: { frontend: {}, backend: {} },
		warnings: { frontend: {}, backend: {} }
	}
}

async function writeFrontendFile(
	workspaceRoot: string | undefined,
	path: string,
	content: string
): Promise<void> {
	if (!workspaceRoot) {
		return
	}
	const relativePath = path.startsWith('/') ? path.slice(1) : path
	const fullPath = join(workspaceRoot, 'frontend', relativePath)
	await mkdir(dirname(fullPath), { recursive: true })
	await writeFile(fullPath, content, 'utf8')
}

async function removeFrontendFile(workspaceRoot: string | undefined, path: string): Promise<void> {
	if (!workspaceRoot) {
		return
	}
	const relativePath = path.startsWith('/') ? path.slice(1) : path
	await rm(join(workspaceRoot, 'frontend', relativePath), { force: true })
}

async function writeBackendRunnable(
	workspaceRoot: string | undefined,
	key: string,
	runnable: BackendRunnable
): Promise<void> {
	if (!workspaceRoot) {
		return
	}
	const runnableDir = join(workspaceRoot, 'backend', key)
	await mkdir(runnableDir, { recursive: true })

	const meta: { name: string; language?: string; type?: string; path?: string } = {
		name: runnable.name
	}

	if (runnable.type === 'inline' && runnable.inlineScript) {
		meta.language = runnable.inlineScript.language
		const extension = runnable.inlineScript.language === 'python3' ? 'py' : 'ts'
		await writeFile(
			join(runnableDir, `main.${extension}`),
			runnable.inlineScript.content,
			'utf8'
		)
	} else {
		meta.type = runnable.type
		if (runnable.path) {
			meta.path = runnable.path
		}
	}

	await writeFile(join(runnableDir, 'meta.json'), JSON.stringify(meta, null, 2) + '\n', 'utf8')
}

async function removeBackendRunnable(workspaceRoot: string | undefined, key: string): Promise<void> {
	if (!workspaceRoot) {
		return
	}
	await rm(join(workspaceRoot, 'backend', key), { recursive: true, force: true })
}

async function persistDatatables(
	workspaceRoot: string | undefined,
	datatables: DataTableSchema[]
): Promise<void> {
	if (!workspaceRoot) {
		return
	}
	await writeFile(
		join(workspaceRoot, 'datatables.json'),
		JSON.stringify(datatables, null, 2) + '\n',
		'utf8'
	)
}

export async function createAppFileHelpers(
	initialFrontend: Record<string, string> = {},
	initialBackend: Record<string, BackendRunnable> = {},
	workspaceRoot?: string
): Promise<{
	helpers: AppAIChatHelpers
	getFiles: () => AppFiles
	getFrontend: () => Record<string, string>
	getBackend: () => Record<string, BackendRunnable>
	cleanup: () => Promise<void>
	workspaceDir: string | null
}> {
	let frontend = { ...initialFrontend }
	let backend = { ...initialBackend }
	let snapshotId = 0
	const snapshots = new Map<
		number,
		{ frontend: Record<string, string>; backend: Record<string, BackendRunnable> }
	>()
	const datatables: DataTableSchema[] = []

	for (const [path, content] of Object.entries(frontend)) {
		await writeFrontendFile(workspaceRoot, path, content)
	}
	for (const [key, runnable] of Object.entries(backend)) {
		await writeBackendRunnable(workspaceRoot, key, runnable)
	}
	await persistDatatables(workspaceRoot, datatables)

	const helpers: AppAIChatHelpers = {
		listFrontendFiles: () => Object.keys(frontend),
		getFrontendFile: (path: string) => frontend[path],
		getFrontendFiles: () => ({ ...frontend }),
		setFrontendFile: (path: string, content: string) => {
			frontend[path] = content
			void writeFrontendFile(workspaceRoot, path, content)
			return createEmptyLintResult()
		},
		deleteFrontendFile: (path: string) => {
			delete frontend[path]
			void removeFrontendFile(workspaceRoot, path)
		},
		listBackendRunnables: () =>
			Object.entries(backend).map(([key, runnable]) => ({
				key,
				name: runnable.name
			})),
		getBackendRunnable: (key: string) => backend[key],
		getBackendRunnables: () => ({ ...backend }),
		setBackendRunnable: async (key: string, runnable: BackendRunnable) => {
			backend[key] = runnable
			await writeBackendRunnable(workspaceRoot, key, runnable)
			return createEmptyLintResult()
		},
		deleteBackendRunnable: (key: string) => {
			delete backend[key]
			void removeBackendRunnable(workspaceRoot, key)
		},
		getFiles: (): AppFiles => ({
			frontend: { ...frontend },
			backend: { ...backend }
		}),
		getSelectedContext: (): SelectedContext => ({ type: 'none' }),
		snapshot: () => {
			const id = ++snapshotId
			snapshots.set(id, {
				frontend: { ...frontend },
				backend: { ...backend }
			})
			return id
		},
		revertToSnapshot: (id: number) => {
			const snapshot = snapshots.get(id)
			if (!snapshot) {
				return
			}
			frontend = { ...snapshot.frontend }
			backend = { ...snapshot.backend }
			void syncWorkspace()
		},
		lint: () => createEmptyLintResult(),
		getDatatables: async () => structuredClone(datatables),
		getAvailableDatatableNames: () => datatables.map((datatable) => datatable.datatable_name),
		execDatatableSql: async (
			datatableName: string,
			sql: string,
			newTable?: { schema: string; name: string }
		) => {
			if (newTable) {
				datatables.push({
					datatable_name: datatableName,
					schemas: {
						[newTable.schema]: {
							[newTable.name]: {}
						}
					}
				})
				await persistDatatables(workspaceRoot, datatables)
			}
			return {
				success: true,
				result: [
					{
						datatableName,
						sql
					}
				]
			}
		},
		addTableToWhitelist: (datatableName: string, schemaName: string, tableName: string) => {
			const existing = datatables.find((entry) => entry.datatable_name === datatableName)
			if (existing) {
				existing.schemas[schemaName] ??= {}
				existing.schemas[schemaName][tableName] ??= {}
			} else {
				datatables.push({
					datatable_name: datatableName,
					schemas: {
						[schemaName]: {
							[tableName]: {}
						}
					}
				})
			}
			void persistDatatables(workspaceRoot, datatables)
		}
	}

	async function syncWorkspace(): Promise<void> {
		if (!workspaceRoot) {
			return
		}
		await rm(join(workspaceRoot, 'frontend'), { recursive: true, force: true })
		await rm(join(workspaceRoot, 'backend'), { recursive: true, force: true })
		for (const [path, content] of Object.entries(frontend)) {
			await writeFrontendFile(workspaceRoot, path, content)
		}
		for (const [key, runnable] of Object.entries(backend)) {
			await writeBackendRunnable(workspaceRoot, key, runnable)
		}
		await persistDatatables(workspaceRoot, datatables)
	}

	return {
		helpers,
		getFiles: () => ({
			frontend: { ...frontend },
			backend: { ...backend }
		}),
		getFrontend: () => ({ ...frontend }),
		getBackend: () => ({ ...backend }),
		cleanup: async () => {
			if (workspaceRoot) {
				await rm(workspaceRoot, { recursive: true, force: true })
			}
		},
		workspaceDir: workspaceRoot ?? null
	}
}
