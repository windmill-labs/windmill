// jsdom has no matchMedia, which Svelte's client build calls at import (svelte/motion).
window.matchMedia ??= (query: string) =>
	({
		matches: false,
		media: query,
		addEventListener() {},
		removeEventListener() {}
	}) as unknown as MediaQueryList
