/**
 * Debug module exports for Python debugging in Windmill.
 *
 * This module provides a minimal DAP (Debug Adapter Protocol) implementation
 * for debugging Python scripts in the Monaco editor.
 *
 * Usage:
 * 1. Start the DAP WebSocket server: python dap_websocket_server.py
 * 2. Import and use MonacoDebugger component in your editor
 *
 * Example:
 * ```svelte
 * <script>
 *   import MonacoDebugger from '$lib/debug/MonacoDebugger.svelte'
 *   let editor // Monaco editor instance
 *   let code = 'print("Hello")'
 * </script>
 *
 * <MonacoDebugger {editor} {code} />
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
