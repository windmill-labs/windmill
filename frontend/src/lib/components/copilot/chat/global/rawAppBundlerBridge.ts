import { WorkspaceService } from '$lib/gen'

export type RawAppBundle = {
	js: string
	css: string
}

type BundleRawAppFilesParams = {
	files: Record<string, string>
	sharedUiFiles?: Record<string, string>
	bundlerType?: 'esbuild' | 'rolldown'
	timeoutMs?: number
	onLog?: (delta: string) => void
}

type BundleRawAppDraftParams = BundleRawAppFilesParams & {
	workspace: string
}

const DEFAULT_TIMEOUT_MS = 120_000

function makeRequestId(): string {
	return globalThis.crypto?.randomUUID?.() ?? Math.random().toString(36).slice(2)
}

async function loadSharedUiFiles(workspace: string): Promise<Record<string, string>> {
	try {
		const res = (await WorkspaceService.getSharedUi({ workspace })) as {
			files?: Record<string, string>
		}
		return res.files ?? {}
	} catch (e) {
		console.warn('Failed to load shared UI for raw app bundling:', e)
		return {}
	}
}

export async function bundleRawAppDraft(params: BundleRawAppDraftParams): Promise<RawAppBundle> {
	const sharedUiFiles = params.sharedUiFiles ?? (await loadSharedUiFiles(params.workspace))
	return bundleRawAppFiles({
		...params,
		sharedUiFiles
	})
}

export function bundleRawAppFiles({
	files,
	sharedUiFiles = {},
	bundlerType = 'esbuild',
	timeoutMs = DEFAULT_TIMEOUT_MS,
	onLog
}: BundleRawAppFilesParams): Promise<RawAppBundle> {
	if (typeof window === 'undefined' || typeof document === 'undefined') {
		return Promise.reject(new Error('Raw app bundling requires a browser environment.'))
	}

	return new Promise((resolve, reject) => {
		const requestId = makeRequestId()
		const iframe = document.createElement('iframe')
		let settled = false
		let bundleRequestSent = false

		const cleanup = () => {
			window.removeEventListener('message', onMessage)
			clearTimeout(timeout)
			iframe.remove()
		}

		const settle = <T>(fn: (value: T) => void, value: T) => {
			if (settled) return
			settled = true
			cleanup()
			fn(value)
		}

		const postToBundler = (message: Record<string, unknown>) => {
			if (!iframe.contentWindow) {
				settle(reject, new Error('Raw app bundler iframe did not initialize.'))
				return
			}
			iframe.contentWindow.postMessage(message, window.location.origin)
		}

		const sendBundleProtocolRequest = () => {
			if (settled || bundleRequestSent) return
			bundleRequestSent = true
			postToBundler({
				type: 'bundleRawApp',
				requestId,
				files,
				sharedUiFiles,
				bundlerType
			})
		}

		const timeout = window.setTimeout(() => {
			settle(reject, new Error('Timed out while bundling raw app.'))
		}, timeoutMs)

		function onMessage(event: MessageEvent) {
			if (event.source !== iframe.contentWindow) return
			const data = event.data
			if (!data) return

			if (data.type === 'bundleRawAppReady') {
				sendBundleProtocolRequest()
				return
			}

			if (data.requestId !== requestId) return

			if (data.type === 'appendLogs') {
				onLog?.(String(data.delta ?? ''))
			} else if (data.type === 'bundleRawAppResult') {
				const bundle = data.bundle
				if (!bundle?.js) {
					settle(reject, new Error('Raw app bundler returned an empty JavaScript bundle.'))
					return
				}
				settle(resolve, {
					js: String(bundle.js),
					css: String(bundle.css ?? '')
				})
			} else if (data.type === 'bundleRawAppError') {
				settle(reject, new Error(String(data.error ?? 'Raw app bundle failed.')))
			}
		}

		iframe.title = 'Raw app bundler'
		iframe.tabIndex = -1
		// Windmill pages use COEP=require-corp; the static UI builder iframe must be credentialless.
		iframe.setAttribute('credentialless', '')
		iframe.style.position = 'fixed'
		iframe.style.width = '0'
		iframe.style.height = '0'
		iframe.style.border = '0'
		iframe.style.opacity = '0'
		iframe.style.pointerEvents = 'none'
		iframe.src = '/ui_builder/index.html?mode=bundle'

		window.addEventListener('message', onMessage)
		document.body.appendChild(iframe)
	})
}
