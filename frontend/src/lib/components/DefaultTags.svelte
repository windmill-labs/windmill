<script lang="ts">
	import { Button, Popup } from './common'
	import { AlertTriangle, Loader2, Pen } from 'lucide-svelte'
	import { SettingService, WorkerService } from '$lib/gen'
	import Tooltip from './Tooltip.svelte'
	import { sendUserToast } from '$lib/toast'
	import { enterpriseLicense, superadmin } from '$lib/stores'
	import { DEFAULT_TAGS_SETTING } from '$lib/consts'

	export let defaultTags: Record<string, string> | undefined = undefined
	export let placement: 'bottom-end' | 'top-end' = 'bottom-end'

	async function loadDefaultTags() {
		console.log('FOO')
		try {
			defaultTags = (await WorkerService.geDefaultTags()) ?? []
		} catch (err) {
			sendUserToast(`Could not load global cache: ${err}`, true)
		}
	}

	loadDefaultTags()
</script>

<Popup
	floatingConfig={{ strategy: 'absolute', placement: placement }}
	containerClasses="border rounded-lg shadow-lg p-4 bg-surface"
>
	<svelte:fragment slot="button">
		<Button color="dark" size="xs" nonCaptureEvent={true}>
			<div class="flex flex-row gap-1 items-center"
				><Pen size={14} /> Default Tags&nbsp;<Tooltip light
					>Scripts and steps that have not been specifically assigned tags will use a default tag
					that can be customized here</Tooltip
				></div
			>
		</Button>
	</svelte:fragment>
	<div class="flex flex-col w-72 p-2 gap-2">
		{#if !$enterpriseLicense}
			<div class="flex text-xs items-center gap-1 text-yellow-500 whitespace-nowrap ml-8">
				<AlertTriangle size={16} />
				EE only <Tooltip>Enterprise Edition only feature</Tooltip>
			</div>
		{/if}
		{#if defaultTags == undefined}
			<Loader2 class="animate-spin" />
		{:else}
			<div class="flex flex-col gap-y-1">
				{#each Object.keys(defaultTags).sort() as tag (tag)}
					<div class="flex gap-2 items-center"
						><div class="p-1 px-2 rounded border text-primary">{tag} </div><div
							class="flex gap-2 items-center"
							>&rightarrow;
							<input
								disabled={!$enterpriseLicense}
								type="text"
								bind:value={defaultTags[tag]}
							/></div
						>
					</div>
				{/each}
			</div>

			<Button
				variant="contained"
				color="blue"
				size="sm"
				on:click={async () => {
					await SettingService.setGlobal({
						key: DEFAULT_TAGS_SETTING,
						requestBody: {
							value: defaultTags
						}
					})
					loadDefaultTags()
					sendUserToast('Saved')
				}}
				disabled={!$enterpriseLicense || !$superadmin}
			>
				Save {#if !$superadmin} <span class="text-2xs text-tertiary">superadmin only</span> {/if}
			</Button>

			<span class="text-2xs text-tertiary"
				>For dynamic tags based on the workspace, use <pre class="inline">$workspace</pre>, e.g:
				<pre class="inline">tag-$workspace</pre></span
			>
		{/if}
	</div>
</Popup>
