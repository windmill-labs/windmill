/**
 * Debug module exports for Python and TypeScript debugging in Windmill.
 *
 * This module provides a minimal DAP (Debug Adapter Protocol) implementation
 * for debugging scripts in the Monaco editor.
 *
 * Supported Languages:
 * - Python: Uses Python's bdb module via dap_websocket_server.py
 * - TypeScript/Bun: Uses V8 Inspector Protocol via dap_websocket_server_bun.ts
 *
 * Usage:
 * 1. Start the appropriate DAP WebSocket server:
 *    - Python: python dap_websocket_server.py --port 5679
 *    - TypeScript: bun run dap_websocket_server_bun.ts --port 5680
 * 2. Import and use MonacoDebugger component in your editor
 *
 * Example:
 * ```svelte
 * <script>
 *   import MonacoDebugger from '$lib/debug/MonacoDebugger.svelte'
 *   let editor // Monaco editor instance
 *   let code = 'console.log("Hello")'
 * </script>
 *
 * <MonacoDebugger
 *   {editor}
 *   {code}
 *   dapServerUrl="ws://localhost:5680"
 *   filePath="/tmp/script.ts"
 * />
 * ```
 */

export { default as MonacoDebugger } from './MonacoDebugger.svelte'
export { default as DebugToolbar } from './DebugToolbar.svelte'
export { default as DebugPanel } from './DebugPanel.svelte'
export {
	DAPClient,
	getDAPClient,
	resetDAPClient,
	debugState,
	type DebugState,
	type Breakpoint,
	type StackFrame,
	type Variable,
	type Scope
} from './dapClient'

/**
 * Default server URLs for each language
 */
export const DAP_SERVER_URLS = {
	python3: 'ws://localhost:5679',
	bun: 'ws://localhost:5680',
	typescript: 'ws://localhost:5680',
	deno: 'ws://localhost:5680' // Same as bun for now
} as const

/**
 * Get the appropriate file extension for a language
 */
export function getDebugFileExtension(language: string): string {
	switch (language) {
		case 'python3':
			return '.py'
		case 'bun':
		case 'typescript':
		case 'deno':
		case 'nativets':
			return '.ts'
		default:
			return '.txt'
	}
}

/**
 * Check if a language supports debugging
 */
export function isDebuggable(language: string): boolean {
	return ['python3', 'bun', 'typescript', 'deno', 'nativets'].includes(language)
}
