<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { ScriptService, type Script } from '$lib/gen'

	import { Wand2, Loader2 } from 'lucide-svelte'
	import SearchItems from '../SearchItems.svelte'
	import { emptyString } from '$lib/utils'
	import { onMount } from 'svelte'

	export let funcDesc: string
	export let trigger = false
	export let loading = false
	export let preFilter: string
	export let disableAi = false

	let scripts: Script[] | undefined = undefined
	export let filteredItems: (Script & { marked?: string })[] | (Item & { marked?: string })[] = []
	$: prefilteredItems = scripts ?? []

	async function loadScripts(): Promise<void> {
		const loadedScripts = await ScriptService.listScripts({
			workspace: $workspaceStore!,
			perPage: 300,
			kinds: trigger ? 'trigger' : 'script'
		})

		scripts = loadedScripts
	}

	$: scripts == undefined && funcDesc?.length > 1 && loadScripts()

	let input: HTMLInputElement

	$: preFilter &&
		setTimeout(() => {
			input?.focus()
		}, 50)

	onMount(() => {
		input?.focus()
	})
</script>

<SearchItems
	filter={funcDesc}
	items={prefilteredItems}
	bind:filteredItems
	f={(x) => (emptyString(x.summary) ? x.path : x.summary + ' (' + x.path + ')')}
/>

<div class="relative text-primary items-center transition-all flex-grow">
	<div class="grow items-cente">
		<input
			bind:this={input}
			type="text"
			bind:value={funcDesc}
			placeholder="Search {trigger ? 'triggers' : 'scripts'} {disableAi ? '' : 'or AI gen'}"
		/>
	</div>
	<div class="absolute inset-y-0 right-3 flex items-center pointer-events-none">
		{#if loading}
			<Loader2 size={16} class="animate-spin text-gray-400" />
		{/if}
		{#if funcDesc?.length === 0 && !loading && !disableAi}
			<Wand2 size={14} class="fill-current opacity-70 text-violet-800 dark:text-violet-400" />
		{/if}
	</div>
</div>
