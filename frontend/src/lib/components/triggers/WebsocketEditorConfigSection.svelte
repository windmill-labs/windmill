<script lang="ts">
	import Section from '$lib/components/Section.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import Required from '$lib/components/Required.svelte'
	import { Loader2 } from 'lucide-svelte'

	export let args: Record<string, any> = { url: '' }
	export let url: string
	export let url_runnable_args: Record<string, unknown> = {}
	export let dirtyUrl: boolean = false
	export let urlError: string = ''
	export let urlRunnableSchema: Record<string, unknown> = {}
	export let can_write: boolean = false
	export let headless: boolean = false
	export let showCapture: boolean = false

	function updateArgs(url: string) {
		args && (args.url = url)
	}

	$: updateArgs(url)
</script>

<Section label="Websocket" {headless}>
	<div class="mb-2">
		<ToggleButtonGroup
			selected={url.startsWith('$') ? 'runnable' : 'static'}
			on:selected={(ev) => {
				url = ev.detail === 'runnable' ? '$script:' : ''
				url_runnable_args = {}
			}}
			disabled={showCapture}
		>
			<ToggleButton value="static" label="Static URL" />
			<ToggleButton value="runnable" label="Runnable result as URL" disabled={showCapture} />
		</ToggleButtonGroup>
	</div>
	{#if url.startsWith('$')}
		<div class="flex flex-col w-full gap-4">
			<div class="block grow w-full">
				<div class="text-secondary text-sm flex items-center gap-1 w-full justify-between">
					<div>
						Runnable
						<Required required={true} />
					</div>
				</div>
				<ScriptPicker
					allowFlow={true}
					itemKind={url.startsWith('$flow:') ? 'flow' : 'script'}
					initialPath={url.split(':')[1] ?? ''}
					on:select={(ev) => {
						dirtyUrl = true
						const { path, itemKind } = ev.detail
						url = `$${itemKind}:${path ?? ''}`
					}}
				/>
				<div class="text-red-600 dark:text-red-400 text-2xs mt-1.5">
					{dirtyUrl ? urlError : ''}
				</div>
			</div>
		</div>

		{#if url.split(':')[1]?.length > 0}
			{#if urlRunnableSchema}
				<p class="font-semibold text-sm mt-4 mb-2">Arguments</p>
				{#await import('$lib/components/SchemaForm.svelte')}
					<Loader2 class="animate-spin mt-2" />
				{:then Module}
					<Module.default
						schema={urlRunnableSchema}
						bind:args={url_runnable_args}
						shouldHideNoInputs
						class="text-xs"
					/>
				{/await}
				{#if urlRunnableSchema.properties && Object.keys(urlRunnableSchema.properties).length === 0}
					<div class="text-xs texg-gray-700">This runnable takes no arguments</div>
				{/if}
			{:else}
				<Loader2 class="animate-spin mt-2" />
			{/if}
		{/if}
	{:else}
		<div class="flex flex-col w-full gap-4">
			<label class="block grow w-full">
				<div class="text-secondary text-sm flex items-center gap-1 w-full justify-between">
					<div>
						URL
						<Required required={true} />
					</div>
				</div>
				<input
					type="text"
					autocomplete="off"
					bind:value={url}
					disabled={!can_write}
					on:input={() => {
						dirtyUrl = true
					}}
					class={urlError === ''
						? ''
						: 'border border-red-700 bg-red-100 border-opacity-30 focus:border-red-700 focus:border-opacity-30 focus-visible:ring-red-700 focus-visible:ring-opacity-25 focus-visible:border-red-700'}
				/>
				<div class="text-red-600 dark:text-red-400 text-2xs mt-1.5">
					{dirtyUrl ? urlError : ''}
				</div>
			</label>
		</div>
	{/if}
</Section>
