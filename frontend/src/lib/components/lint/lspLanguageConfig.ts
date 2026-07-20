import { buildWsUrl } from '$lib/wsUrl'

// Which language servers back a given language, and with which initialization options.
// Diagnostics are decided by the server plus these options, so a headless linter and a
// mounted editor only agree as long as both build their clients from here.

export interface LspServerConfig {
	name: string
	url: string
	initOptions: any
	middleware?: (params: any, token: any, next: any) => any
}

export function lspLanguagesFor(editorLang: string, scriptLang: string | undefined): boolean {
	return (
		(editorLang === 'typescript' && scriptLang === 'deno') ||
		editorLang === 'python' ||
		editorLang === 'go' ||
		editorLang === 'shell'
	)
}

/** Deno resolves relative imports through an import map rooted at the tokened raw endpoint. */
export function buildDenoImportMap(root: string, filePath: string | undefined): string {
	const importMap: { imports: Record<string, string> } = {
		imports: {
			'file:///': root + '/'
		}
	}
	if (filePath && filePath.split('/').length > 2) {
		const path_splitted = filePath.split('/')
		for (let c = 0; c < path_splitted.length; c++) {
			let key = 'file://./'
			for (let i = 0; i < c; i++) {
				key += '../'
			}
			const url = path_splitted.slice(0, -c - 1).join('/')
			const ending = c == path_splitted.length - 1 ? '' : '/'
			importMap['imports'][key] = `${root}/${url}${ending}`
		}
	}
	return 'data:text/plain;base64,' + btoa(JSON.stringify(importMap))
}

function denoServer(encodedImportMap: string): LspServerConfig {
	return {
		name: 'deno',
		url: buildWsUrl('/ws/deno'),
		initOptions: {
			certificateStores: null,
			enablePaths: [],
			config: null,
			importMap: encodedImportMap,
			internalDebug: false,
			lint: false,
			path: null,
			tlsCertificate: null,
			unsafelyIgnoreCertificateErrors: null,
			unstable: true,
			enable: true,
			codeLens: {
				implementations: true,
				references: true,
				referencesAllFunction: false
			},
			suggest: {
				autoImports: true,
				completeFunctionCalls: false,
				names: true,
				paths: true,
				imports: {
					autoDiscover: true,
					hosts: {
						'https://deno.land': true
					}
				}
			}
		},
		middleware: () => [{ enable: true }]
	}
}

const PYTHON_ANALYSIS_SETTINGS = {
	useLibraryCodeForTypes: true,
	autoImportCompletions: true,
	diagnosticSeverityOverrides: { reportMissingImports: 'none' },
	typeCheckingMode: 'basic'
}

const pyrightServer: LspServerConfig = {
	name: 'pyright',
	url: buildWsUrl('/ws/pyright'),
	initOptions: {},
	middleware: (params, token, next) => {
		if (params.items.find((x) => x.section === 'python')) {
			return [{ analysis: PYTHON_ANALYSIS_SETTINGS }]
		}
		if (params.items.find((x) => x.section === 'python.analysis')) {
			return [PYTHON_ANALYSIS_SETTINGS]
		}
		return next(params, token)
	}
}

const ruffServer: LspServerConfig = {
	name: 'ruff',
	url: buildWsUrl('/ws/ruff'),
	initOptions: {}
}

const goServer: LspServerConfig = {
	name: 'go',
	url: buildWsUrl('/ws/go'),
	initOptions: { 'build.allowImplicitNetworkAccess': true }
}

const shellcheckServer: LspServerConfig = {
	name: 'shellcheck',
	url: buildWsUrl('/ws/diagnostic'),
	initOptions: {
		linters: {
			shellcheck: {
				command: 'shellcheck',
				debounce: 100,
				args: ['--format=gcc', '-'],
				offsetLine: 0,
				offsetColumn: 0,
				sourceName: 'shellcheck',
				formatLines: 1,
				formatPattern: [
					'^[^:]+:(\\d+):(\\d+):\\s+([^:]+):\\s+(.*)$',
					{
						line: 1,
						column: 2,
						message: 4,
						security: 3
					}
				],
				securities: {
					error: 'error',
					warning: 'warning',
					note: 'info'
				}
			}
		},
		filetypes: {
			shell: 'shellcheck'
		}
	}
}

export function lspServersFor(opts: {
	editorLang: string
	scriptLang: string | undefined
	denoImportMap?: string
}): LspServerConfig[] {
	if (opts.editorLang === 'typescript' && opts.scriptLang === 'deno') {
		return [denoServer(opts.denoImportMap ?? '')]
	}
	if (opts.editorLang === 'python') {
		return [pyrightServer, ruffServer]
	}
	if (opts.editorLang === 'go') {
		return [goServer]
	}
	if (opts.editorLang === 'shell') {
		return [shellcheckServer]
	}
	return []
}
