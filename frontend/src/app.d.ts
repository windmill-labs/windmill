/// <reference types="@sveltejs/kit" />

// See https://kit.svelte.dev/docs/types#app
// for information about these interfaces
declare namespace App {
	interface Stuff {
		title: string?
	}
}

declare var __pkg__: { version: string }
/** The readable session-hint cookie's name: `wm_session`, renamed per backend in dev (vite.config.js). */
declare var __WM_SESSION_HINT__: string

declare module 'svelte-highlight/languages/json'
declare module 'svelte-highlight/languages/python'
declare module 'svelte-highlight/languages/typescript'
declare module 'svelte-highlight/styles/github'
