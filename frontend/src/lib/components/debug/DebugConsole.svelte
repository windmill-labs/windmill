<script lang="ts">
	import { X, AlertCircle, Trash2 } from 'lucide-svelte'
	import type { DAPClient } from './dapClient'

	interface Props {
		client: DAPClient | null
		currentFrameId?: number
		onClose?: () => void
		/** Workspace ID for signing expressions (audit logging) */
		workspace?: string
		/** Job ID of the parent debug session (for expression signing) */
		jobId?: string
	}

	interface ConsoleEntry {
		id: number
		type: 'input' | 'output' | 'error'
		content: string
		timestamp: Date
	}

	let { client, currentFrameId, onClose, workspace, jobId }: Props = $props()

	let inputValue = $state('')
	let history: ConsoleEntry[] = $state([])
	let commandHistory: string[] = $state([])
	let historyIndex = $state(-1)
	let isEvaluating = $state(false)
	let nextId = $state(1)
	let consoleRef: HTMLDivElement | null = $state(null)
	let inputRef: HTMLInputElement | null = $state(null)

	/**
	 * Sign an expression for audit logging before evaluation.
	 * Returns the JWT token if signing succeeds, or undefined if signing is not available.
	 */
	async function signExpression(expression: string): Promise<string | undefined> {
		if (!workspace || !jobId) {
			// If workspace or jobId not provided, skip signing (for backwards compatibility)
			return undefined
		}

		try {
			const response = await fetch(`/api/w/${workspace}/debug/sign_expression`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ expression, job_id: jobId })
			})

			if (!response.ok) {
				console.warn('Failed to sign expression:', await response.text())
				// Don't block evaluation if signing fails - just log it
				return undefined
			}

			const result = await response.json()
			return result.token
		} catch (error) {
			console.warn('Failed to sign expression:', error)
			// Don't block evaluation if signing fails - just log it
			return undefined
		}
	}

	// Focus input - only when explicitly requested (not on every click)
	function focusInput(): void {
		// Use requestAnimationFrame to ensure DOM is ready
		requestAnimationFrame(() => {
			inputRef?.focus()
		})
	}

	// Handle click on container - only focus if clicking on empty background areas
	function handleContainerClick(e: MouseEvent): void {
		// Don't focus if user has selected text
		const selection = window.getSelection()
		if (selection && selection.toString().length > 0) {
			return
		}
		// Only focus if clicking directly on container divs, not on text content
		const target = e.target as HTMLElement
		// Skip if clicking on interactive elements
		if (target.tagName === 'BUTTON' || target.tagName === 'INPUT' || target.closest('button')) {
			return
		}
		// Skip if clicking on text content (span elements with output)
		if (target.tagName === 'SPAN' && target.textContent && target.textContent.trim().length > 0) {
			return
		}
		focusInput()
	}

	async function evaluate(): Promise<void> {
		const expression = inputValue.trim()
		if (!expression || !client || isEvaluating) return

		// Add to command history
		if (commandHistory[commandHistory.length - 1] !== expression) {
			commandHistory = [...commandHistory, expression]
		}
		historyIndex = -1

		// Add input entry
		history = [
			...history,
			{
				id: nextId++,
				type: 'input',
				content: expression,
				timestamp: new Date()
			}
		]

		inputValue = ''
		isEvaluating = true

		try {
			// Sign the expression for audit logging (if workspace and jobId are available)
			const token = await signExpression(expression)

			// Evaluate with the signed token
			const result = await client.evaluate(expression, currentFrameId, 'repl', token)
			history = [
				...history,
				{
					id: nextId++,
					type: 'output',
					content: result.result ?? 'undefined',
					timestamp: new Date()
				}
			]
		} catch (error) {
			history = [
				...history,
				{
					id: nextId++,
					type: 'error',
					content: error instanceof Error ? error.message : String(error),
					timestamp: new Date()
				}
			]
		} finally {
			isEvaluating = false
			// Scroll to bottom and refocus
			setTimeout(() => {
				if (consoleRef) {
					consoleRef.scrollTop = consoleRef.scrollHeight
				}
				focusInput()
			}, 0)
		}
	}

	function handleKeydown(event: KeyboardEvent): void {
		if (event.key === 'Enter' && !event.shiftKey) {
			event.preventDefault()
			evaluate()
		} else if (event.key === 'ArrowUp') {
			event.preventDefault()
			if (commandHistory.length > 0) {
				if (historyIndex === -1) {
					historyIndex = commandHistory.length - 1
				} else if (historyIndex > 0) {
					historyIndex--
				}
				inputValue = commandHistory[historyIndex]
			}
		} else if (event.key === 'ArrowDown') {
			event.preventDefault()
			if (historyIndex !== -1) {
				if (historyIndex < commandHistory.length - 1) {
					historyIndex++
					inputValue = commandHistory[historyIndex]
				} else {
					historyIndex = -1
					inputValue = ''
				}
			}
		} else if (event.key === 'l' && event.ctrlKey) {
			event.preventDefault()
			clearConsole()
		} else if (event.key === 'Escape') {
			event.preventDefault()
			onClose?.()
		}
	}

	function clearConsole(): void {
		history = []
		focusInput()
	}

	// Format value for display (Chrome-like)
	function formatValue(content: string, type: 'output' | 'error'): string {
		if (type === 'error') return content

		// Try to detect type for coloring
		if (content === 'undefined' || content === 'null') return content
		if (content === 'true' || content === 'false') return content
		if (/^-?\d+(\.\d+)?$/.test(content)) return content
		if (content.startsWith('"') && content.endsWith('"')) return content

		return content
	}

	function getValueClass(content: string, type: 'output' | 'error'): string {
		if (type === 'error') return 'text-red-500'
		if (content === 'undefined' || content === 'null') return 'text-gray-500'
		if (content === 'true' || content === 'false') return 'text-blue-600 dark:text-blue-400'
		if (/^-?\d+(\.\d+)?$/.test(content)) return 'text-blue-600 dark:text-blue-400'
		if (content.startsWith('"') && content.endsWith('"')) return 'text-red-600 dark:text-red-400'
		if (content.startsWith('{') || content.startsWith('[')) return 'text-primary'
		return 'text-primary'
	}
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="flex flex-col h-full bg-[#242424] text-[#d4d4d4] font-mono text-xs select-text"
	onclick={handleContainerClick}
>
	<!-- Header -->
	<div class="flex items-center justify-between px-2 py-1 bg-[#1e1e1e] border-b border-[#3c3c3c]">
		<span class="text-[11px] text-[#969696]">Console</span>
		<div class="flex items-center gap-1">
			<button
				class="p-0.5 hover:bg-[#3c3c3c] rounded text-[#969696] hover:text-[#d4d4d4]"
				onclick={(e) => { e.stopPropagation(); clearConsole(); }}
				title="Clear console (Ctrl+L)"
			>
				<Trash2 size={12} />
			</button>
			{#if onClose}
				<button
					class="p-0.5 hover:bg-[#3c3c3c] rounded text-[#969696] hover:text-[#d4d4d4]"
					onclick={(e) => { e.stopPropagation(); onClose?.(); }}
					title="Close console (Esc)"
				>
					<X size={14} />
				</button>
			{/if}
		</div>
	</div>

	<!-- Console output -->
	<div
		bind:this={consoleRef}
		class="flex-1 overflow-auto min-h-0"
	>
		{#if history.length === 0}
			<div class="px-3 py-2 text-[#969696] text-[11px]">
				Evaluate expressions in the current scope. Use ↑↓ for history.
			</div>
		{/if}
		{#each history as entry (entry.id)}
			<div
				class="flex items-start px-2 py-0.5 border-b border-[#3c3c3c]/50 hover:bg-[#2a2a2a]"
				class:bg-[#332222]={entry.type === 'error'}
			>
				{#if entry.type === 'input'}
					<span class="text-[#569cd6] mr-2 select-none">&gt;</span>
					<span class="text-[#ce9178] break-all whitespace-pre-wrap">{entry.content}</span>
				{:else if entry.type === 'output'}
					<span class="text-[#569cd6] mr-2 select-none opacity-0">&gt;</span>
					<span class="{getValueClass(entry.content, 'output')} break-all whitespace-pre-wrap">{formatValue(entry.content, 'output')}</span>
				{:else}
					<AlertCircle size={12} class="text-red-500 mr-2 mt-0.5 flex-shrink-0" />
					<span class="text-red-400 break-all whitespace-pre-wrap">{entry.content}</span>
				{/if}
			</div>
		{/each}
	</div>

	<!-- Input line -->
	<div class="flex items-center px-2 py-1 border-t border-[#3c3c3c] bg-[#1e1e1e]">
		<span class="text-[#569cd6] mr-2 select-none">&gt;</span>
		<input
			bind:this={inputRef}
			type="text"
			bind:value={inputValue}
			onkeydown={handleKeydown}
			placeholder={client ? '' : 'Waiting for debugger...'}
			disabled={!client}
			class="flex-1 bg-transparent text-[#d4d4d4] placeholder-[#6e6e6e] focus:outline-none disabled:opacity-50"
			autocomplete="off"
			autocorrect="off"
			autocapitalize="off"
			spellcheck="false"
		/>
		{#if isEvaluating}
			<div class="w-3 h-3 border border-[#569cd6] border-t-transparent rounded-full animate-spin ml-2"></div>
		{/if}
	</div>
</div>
