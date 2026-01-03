<script lang="ts">
	import { onMount, onDestroy } from 'svelte'
	import { SvelteSet } from 'svelte/reactivity'
	import type { editor as meditor, IDisposable } from 'monaco-editor'
	import { debugState, getDAPClient, resetDAPClient, type DAPClient } from './dapClient'
	import DebugToolbar from './DebugToolbar.svelte'
	import DebugPanel from './DebugPanel.svelte'
	import { DAP_SERVER_URLS, getDebugFileExtension, type DebugLanguage } from './index'

	interface Props {
		editor: meditor.IStandaloneCodeEditor | null
		code: string
		language?: DebugLanguage
		filePath?: string
		dapServerUrl?: string
	}

	let { editor, code, language = 'bun', filePath, dapServerUrl }: Props = $props()

	// Derive the server URL from language if not explicitly provided
	const effectiveServerUrl = $derived(dapServerUrl ?? DAP_SERVER_URLS[language])
	// Derive file path from language if not explicitly provided
	const effectiveFilePath = $derived(filePath ?? `/tmp/script${getDebugFileExtension(language)}`)

	let client: DAPClient | null = $state(null)
	let breakpointDecorations: string[] = $state([])
	let currentLineDecoration: string[] = $state([])
	let disposables: IDisposable[] = []

	// Breakpoint glyph margin decoration
	const breakpointDecorationType: meditor.IModelDecorationOptions = {
		glyphMarginClassName: 'debug-breakpoint-glyph',
		glyphMarginHoverMessage: { value: 'Breakpoint' },
		stickiness: 1 // NeverGrowsWhenTypingAtEdges
	}

	// Current line decoration (yellow background when stopped)
	const currentLineDecorationType: meditor.IModelDecorationOptions = {
		isWholeLine: true,
		className: 'debug-current-line',
		glyphMarginClassName: 'debug-current-line-glyph'
	}

	// Track breakpoints by line number
	let breakpoints = new SvelteSet<number>()

	// Track the last used server URL to detect changes
	let lastServerUrl: string | undefined = undefined

	// React to language/server URL changes - fully exit debug mode and reset client
	$effect(() => {
		const newUrl = effectiveServerUrl
		if (lastServerUrl !== undefined && lastServerUrl !== newUrl) {
			// Server URL changed (language switched), fully exit debug mode
			console.log('[DAP] Language changed, switching from', lastServerUrl, 'to', newUrl)

			// Terminate and disconnect if we have an active session
			if (client) {
				if (client.isConnected()) {
					// Try to terminate gracefully, then disconnect
					client.terminate().catch(() => {}).finally(() => {
						client?.disconnect()
					})
				}
			}

			// Reset the singleton and clear local client reference
			resetDAPClient()
			client = null

			// Clear current line decoration since we're exiting debug mode
			if (editor) {
				currentLineDecoration = editor.deltaDecorations(currentLineDecoration, [])
			}
		}
		lastServerUrl = newUrl
	})

	// Export function to refresh breakpoint positions - called by parent when code changes
	export function refreshBreakpoints(): void {
		updateBreakpointPositionsFromDecorations()
	}

	onMount(() => {
		if (!editor) return

		client = getDAPClient(effectiveServerUrl)
		lastServerUrl = effectiveServerUrl

		// Add click handler for glyph margin (breakpoint toggle)
		const mouseDownDisposable = editor.onMouseDown((e) => {
			if (e.target.type === 2) {
				// MouseTargetType.GUTTER_GLYPH_MARGIN
				const line = e.target.position?.lineNumber
				if (line) {
					toggleBreakpoint(line)
				}
			}
		})
		disposables.push(mouseDownDisposable)

		// Add keyboard shortcut F9 for toggling breakpoint
		editor.addCommand(
			120, // KeyCode.F9
			() => {
				const position = editor?.getPosition()
				if (position) {
					toggleBreakpoint(position.lineNumber)
				}
			}
		)

		// Update decorations when state changes
		const unsubscribe = debugState.subscribe((state) => {
			updateCurrentLineDecoration(state.currentLine)
		})

		return () => {
			unsubscribe()
		}
	})

	onDestroy(() => {
		disposables.forEach((d) => d.dispose())
		disposables = []
	})

	function toggleBreakpoint(line: number): void {
		if (breakpoints.has(line)) {
			breakpoints.delete(line)
		} else {
			breakpoints.add(line)
		}
		// SvelteSet is reactive, no need to reassign
		updateBreakpointDecorations()
		syncBreakpointsWithServer()
	}

	function updateBreakpointDecorations(): void {
		if (!editor) return

		const model = editor.getModel()
		if (!model) return

		const decorations: meditor.IModelDeltaDecoration[] = Array.from(breakpoints).map((line) => ({
			range: { startLineNumber: line, startColumn: 1, endLineNumber: line, endColumn: 1 },
			options: breakpointDecorationType
		}))

		breakpointDecorations = editor.deltaDecorations(breakpointDecorations, decorations)
	}

	// Update breakpoint line numbers from decoration positions after code edits
	function updateBreakpointPositionsFromDecorations(): void {
		console.log('[DAP] updateBreakpointPositionsFromDecorations called, decorations:', breakpointDecorations.length)
		if (!editor || breakpointDecorations.length === 0) return

		const model = editor.getModel()
		if (!model) return

		// Get current line numbers from decorations (using plain Set as this is not reactive state)
		const newLines: Set<number> = new Set()
		for (const decorationId of breakpointDecorations) {
			const range = model.getDecorationRange(decorationId)
			if (range) {
				newLines.add(range.startLineNumber)
			}
		}

		// Check if positions changed
		const oldLines = Array.from(breakpoints).sort((a, b) => a - b)
		const updatedLines = Array.from(newLines).sort((a, b) => a - b)

		console.log('[DAP] Old breakpoint lines:', oldLines, 'New lines from decorations:', updatedLines)

		const positionsChanged =
			oldLines.length !== updatedLines.length ||
			oldLines.some((line, i) => line !== updatedLines[i])

		if (positionsChanged) {
			console.log('[DAP] Breakpoint positions changed, syncing with server')
			// Update breakpoints set with new positions
			breakpoints.clear()
			for (const line of newLines) {
				breakpoints.add(line)
			}
			// Sync updated positions with server
			syncBreakpointsWithServer()
		}
	}

	function updateCurrentLineDecoration(line: number | undefined): void {
		if (!editor) return

		if (!line) {
			currentLineDecoration = editor.deltaDecorations(currentLineDecoration, [])
			return
		}

		const decorations: meditor.IModelDeltaDecoration[] = [
			{
				range: { startLineNumber: line, startColumn: 1, endLineNumber: line, endColumn: 1 },
				options: currentLineDecorationType
			}
		]

		currentLineDecoration = editor.deltaDecorations(currentLineDecoration, decorations)

		// Scroll to the current line
		editor.revealLineInCenter(line)
	}

	async function syncBreakpointsWithServer(): Promise<void> {
		console.log('[DAP] syncBreakpointsWithServer called, connected:', client?.isConnected(), 'breakpoints:', Array.from(breakpoints))
		if (!client || !client.isConnected()) {
			console.log('[DAP] Not syncing - client not connected')
			return
		}

		try {
			console.log('[DAP] Sending setBreakpoints to server:', effectiveFilePath, Array.from(breakpoints))
			await client.setBreakpoints(effectiveFilePath, Array.from(breakpoints))
			console.log('[DAP] setBreakpoints completed')
		} catch (error) {
			console.error('Failed to sync breakpoints:', error)
		}
	}

	async function startDebugging(): Promise<void> {
		// Always reset and create a fresh client with the current server URL
		// This ensures we connect to the correct endpoint for the current language
		resetDAPClient()
		client = getDAPClient(effectiveServerUrl)

		try {
			await client.connect()
			await client.initialize()
			await client.setBreakpoints(effectiveFilePath, Array.from(breakpoints))
			await client.configurationDone()
			await client.launch({ code, cwd: '/tmp' })
		} catch (error) {
			console.error('Failed to start debugging:', error)
		}
	}

	async function stopDebugging(): Promise<void> {
		if (!client) return

		try {
			await client.terminate()
			client.disconnect()
		} catch (error) {
			console.error('Failed to stop debugging:', error)
		}
	}

	async function continueExecution(): Promise<void> {
		if (!client) return
		await client.continue_()
	}

	async function stepOver(): Promise<void> {
		if (!client) return
		await client.stepOver()
	}

	async function stepIn(): Promise<void> {
		if (!client) return
		await client.stepIn()
	}

	async function stepOut(): Promise<void> {
		if (!client) return
		await client.stepOut()
	}

	function clearAllBreakpoints(): void {
		breakpoints.clear()
		updateBreakpointDecorations()
		syncBreakpointsWithServer()
	}
</script>

<style>
	:global(.debug-breakpoint-glyph) {
		background-color: #e51400;
		border-radius: 50%;
		width: 10px !important;
		height: 10px !important;
		margin-left: 5px;
		margin-top: 4px;
	}

	:global(.debug-current-line) {
		background-color: rgba(255, 238, 0, 0.2);
	}

	:global(.debug-current-line-glyph) {
		background-color: #ffcc00;
		clip-path: polygon(0 0, 100% 50%, 0 100%);
		width: 10px !important;
		height: 14px !important;
		margin-left: 5px;
		margin-top: 2px;
	}
</style>

<div class="flex flex-col h-full">
	<DebugToolbar
		connected={$debugState.connected}
		running={$debugState.running}
		stopped={$debugState.stopped}
		breakpointCount={breakpoints.size}
		onStart={startDebugging}
		onStop={stopDebugging}
		onContinue={continueExecution}
		onStepOver={stepOver}
		onStepIn={stepIn}
		onStepOut={stepOut}
		onClearBreakpoints={clearAllBreakpoints}
	/>

	{#if $debugState.connected}
		<DebugPanel
			stackFrames={$debugState.stackFrames}
			scopes={$debugState.scopes}
			variables={$debugState.variables}
			{client}
		/>
	{/if}
</div>
