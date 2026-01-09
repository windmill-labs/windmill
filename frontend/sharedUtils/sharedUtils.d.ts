/// <reference types="vite/client" />

// Vite environment variables
interface ImportMetaEnv {
	readonly VITE_APP_TITLE: string
	// Add other env variables as needed
	readonly REMOTE?: string
	readonly REMOTE_LSP?: string
}

interface ImportMeta {
	readonly env: ImportMetaEnv
}

// Global __pkg__ variable from Vite's define
declare const __pkg__: {
	version: string
}

// Svelte component imports
declare module '*.svelte' {
	import type { ComponentType } from 'svelte'
	const component: ComponentType
	export default component
	// Add named exports that your code uses
	export const Runnable: ComponentType
	export const RunsSelectionMode: ComponentType
}

// JSON raw imports
declare module '*.json?raw' {
	const content: string
	export default content
}

// For other raw imports if needed
declare module '*?raw' {
	const content: string
	export default content
}
