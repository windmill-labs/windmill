/// <reference types="@sveltejs/kit" />

// See https://kit.svelte.dev/docs/types#app
// for information about these interfaces
declare namespace App {
	interface Stuff {
		title: string?
	}
}

declare var __pkg__: { version: string }

declare module 'svelte-highlight/languages/json'
declare module 'svelte-highlight/languages/python'
declare module 'svelte-highlight/languages/typescript'
declare module 'svelte-highlight/styles/github'
