/**
 * Debug module exports for Python and TypeScript debugging in Windmill.
 *
 * This module provides a minimal DAP (Debug Adapter Protocol) implementation
 * for debugging scripts in the Monaco editor.
 *
 * Supported Languages:
 * - Python: Uses debugpy via the unified DAP Debug Service
 * - TypeScript/Bun: Uses V8 Inspector Protocol via the unified DAP Debug Service
 *
 * Usage:
 * 1. Start the unified DAP Debug Service:
 *    bun run src/lib/debug/dap_debug_service.ts
 *
 * 2. Import and use MonacoDebugger component in your editor
 *
 * The service provides path-based routing:
 * - /python     - Python debugging via debugpy
 * - /typescript - TypeScript/Bun debugging via WebKit Inspector
 * - /bun        - Alias for /typescript
 *
 * Example:
 * ```svelte
 * <script>
 *   import MonacoDebugger from '$lib/debug/MonacoDebugger.svelte'
 *   let editor // Monaco editor instance
 *   let code = 'console.log("Hello")'
 * </script>
 *
 * <!-- Option 1: Use language prop (recommended) -->
 * <MonacoDebugger {editor} {code} language="bun" />
 *
 * <!-- Option 2: Explicit URL and path -->
 * <MonacoDebugger
 *   {editor}
 *   {code}
 *   dapServerUrl="ws://localhost:5679/typescript"
 *   filePath="/tmp/script.ts"
 * />
 * ```
 */

export { default as MonacoDebugger } from './MonacoDebugger.svelte'
export { default as DebugToolbar } from './DebugToolbar.svelte'
export { default as DebugPanel } from './DebugPanel.svelte'
export { default as DebugVariableViewer } from './DebugVariableViewer.svelte'
export { default as DebugConsole } from './DebugConsole.svelte'
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
 * Default server URLs for each language.
 * Uses the unified DAP Debug Service with path-based routing.
 */
export const DAP_SERVER_URLS = {
	python3: 'ws://localhost:5679/python',
	bun: 'ws://localhost:5679/bun',
	typescript: 'ws://localhost:5679/typescript',
	nativets: 'ws://localhost:5679/typescript',
	deno: 'ws://localhost:5679/typescript'
} as const

/**
 * Supported debug languages
 */
export type DebugLanguage = keyof typeof DAP_SERVER_URLS

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
