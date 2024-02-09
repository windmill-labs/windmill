<script lang="ts">
	import { Button, Popup } from './common'
	import { ExternalLink, Loader2, Pen, X } from 'lucide-svelte'
	import { SettingService, WorkerService } from '$lib/gen'
	import Tooltip from './Tooltip.svelte'
	import { sendUserToast } from '$lib/toast'
	import { superadmin } from '$lib/stores'
	import NoWorkerWithTagWarning from './runs/NoWorkerWithTagWarning.svelte'
	import { CUSTOM_TAGS_SETTING, DEFAULT_TAGS_SETTING } from '$lib/consts'

	let newTag: string = ''

	export let defaultTags: Record<string, string> | undefined = undefined
	export let placement: 'bottom-end' | 'top-end' = 'bottom-end'

	async function loadCustomTags() {
		try {
			defaultTags = (await WorkerService.geDefaultTags()) ?? []
		} catch (err) {
			sendUserToast(`Could not load global cache: ${err}`, true)
		}
	}

	$: if ($superadmin) {
		loadCustomTags()
	}
</script>

<Popup
	floatingConfig={{ strategy: 'absolute', placement: placement }}
	containerClasses="border rounded-lg shadow-lg p-4 bg-surface"
>
	<svelte:fragment slot="button">
		<Button color="dark" size="xs" nonCaptureEvent={true}>
			<div class="flex flex-row gap-1 items-center"
				><Pen size={14} /> Assignable tags&nbsp;<Tooltip light
					>Tags are assigned to scripts and flows. Workers only accept jobs that correspond to their
					worker tags. Scripts have a default tag based on the language they are in but users can
					choose to override their tags with custom ones. This editor allow you to set the custom
					tags one can override the scripts and flows with.</Tooltip
				></div
			>
		</Button>
	</svelte:fragment>
	<div class="flex flex-col w-72 p-2 gap-2">
		{#if defaultTags == undefined}
			<Loader2 class="animate-spin" />
		{:else}
			<div class="flex flex-col gap-y-1">
				{#each Object.entries(defaultTags) as customTag}
					<div class="flex gap-0.5 items-center"
						><div class="text-2xs p-1 rounded border text-primary">{customTag}</div>
						<button
							class="z-10 rounded-full p-1 duration-200 hover:bg-gray-200"
							aria-label="Remove item"
							on:click|preventDefault|stopPropagation={async () => {
								await SettingService.setGlobal({
									key: DEFAULT_TAGS_SETTING,
									requestBody: { value: defaultTags?.filter((x) => x != customTag) }
								})
								loadCustomTags()
								sendUserToast('Tag removed')
							}}
						>
							<X size={12} />
						</button>
					</div>
				{/each}
			</div>

			<span class="text-sm text-primary"
				>Configure <a href="/workers" target="_blank" class="inline-flex gap-1 items-baseline"
					>worker groups <ExternalLink size={12} /></a
				> to listen to tags</span
			>
			<span class="text-2xs text-tertiary"
				>For tags specific to some workspaces, use <pre class="inline"
					>tag(workspace1+workspace2)</pre
				></span
			>
			<span class="text-2xs text-tertiary"
				>For dynamic tags based on the workspace, use <pre class="inline">$workspace</pre>, e.g:
				<pre class="inline">tag-$workspace</pre></span
			>
		{/if}
	</div>
</Popup>
