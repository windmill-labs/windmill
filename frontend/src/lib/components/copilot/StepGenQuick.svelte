<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { ScriptService, type Script } from '$lib/gen'

	import { Wand2, Loader2 } from 'lucide-svelte'
	import SearchItems from '../SearchItems.svelte'
	import { emptyString } from '$lib/utils'
	import { createEventDispatcher, onMount, untrack } from 'svelte'
	import TextInput from '../text_input/TextInput.svelte'

	let scripts: Script[] | undefined = $state(undefined)
	interface Props {
		funcDesc: string
		trigger?: boolean
		loading?: boolean
		preFilter: string
		disableAi?: boolean
		filteredItems?: (Script & { marked?: string })[] | (Item & { marked?: string })[]
	}

	let {
		funcDesc = $bindable(),
		trigger = false,
		loading = false,
		preFilter,
		disableAi = false,
		filteredItems = $bindable([])
	}: Props = $props()
	let prefilteredItems = $derived(scripts ?? [])

	const dispatch = createEventDispatcher()

	async function loadScripts(): Promise<void> {
		const loadedScripts = await ScriptService.listScripts({
			workspace: $workspaceStore!,
			perPage: 300,
			kinds: trigger ? 'trigger' : 'script'
		})

		scripts = loadedScripts
	}

	$effect(() => {
		scripts == undefined && funcDesc?.length > 1 && untrack(() => loadScripts())
	})

	let input: TextInput | undefined = $state()

	$effect(() => {
		preFilter &&
			setTimeout(() => {
				input?.focus()
			}, 50)
	})

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
		<TextInput
			bind:this={input}
			bind:value={funcDesc}
			class="pr-7"
			inputProps={{
				type: 'text',
				onkeydown: (e) => {
					if (e.key === 'Escape') dispatch('escape')
				},
				placeholder: `Search ${trigger ? 'triggers' : 'scripts'} ${disableAi ? '' : 'or AI gen'}`
			}}
			size="sm"
		/>
	</div>
	<div class="absolute inset-y-0 right-3 flex items-center pointer-events-none">
		{#if loading}
			<Loader2 size={12} class="animate-spin text-gray-400" />
		{/if}
		{#if funcDesc?.length === 0 && !loading && !disableAi}
			<Wand2 size={12} class="fill-current opacity-70 text-ai" />
		{/if}
	</div>
</div>
