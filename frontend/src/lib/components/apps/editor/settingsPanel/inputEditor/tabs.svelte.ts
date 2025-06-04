import type { Writable } from 'svelte/store'

export type TabsContext = {
	selected: Writable<string>
	update: (value: string) => void
	hashNavigation: boolean
}
