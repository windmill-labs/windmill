<script lang="ts">
	import { preventDefault, stopPropagation } from 'svelte/legacy'

	import { Button } from './common'
	import { ExternalLink, Loader2, X } from 'lucide-svelte'
	import { SettingService, WorkerService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { superadmin, devopsRole } from '$lib/stores'
	import NoWorkerWithTagWarning from './runs/NoWorkerWithTagWarning.svelte'
	import { CUSTOM_TAGS_SETTING } from '$lib/consts'
	import { base } from '$lib/base'
	import { createEventDispatcher } from 'svelte'

	interface Props {
		showWorkspaceRestriction?: boolean
	}

	let { showWorkspaceRestriction = false }: Props = $props()
	let newTag: string = $state('')
	let customTags: string[] | undefined = $state(undefined)

	async function loadCustomTags() {
		try {
			customTags =
				(await WorkerService.getCustomTags({
					showWorkspaceRestriction
				})) ?? []
		} catch (err) {
			sendUserToast(`Could not load global cache: ${err}`, true)
		}
	}

	const dispatch = createEventDispatcher()

	loadCustomTags()
</script>

<div class="flex flex-col w-72 p-4 gap-2">
	{#if customTags == undefined}
		<Loader2 class="animate-spin" />
	{:else}
		<div class="flex flex-col gap-y-1">
			{#each customTags as customTag}
				<div class="flex gap-0.5 items-center"
					><div class="text-2xs p-1 rounded border text-primary">{customTag}</div>
					<button
						class="z-10 rounded-full p-1 duration-200 hover:bg-gray-200"
						aria-label="Remove item"
						onclick={stopPropagation(
							preventDefault(async () => {
								await SettingService.setGlobal({
									key: CUSTOM_TAGS_SETTING,
									requestBody: { value: customTags?.filter((x) => x != customTag) }
								})
								dispatch('refresh')
								loadCustomTags()
								sendUserToast('Tag removed')
							})
						)}
					>
						<X size={12} />
					</button><NoWorkerWithTagWarning tag={customTag} />
				</div>
			{/each}
		</div>
		<input type="text" bind:value={newTag} />

		<Button
			variant="contained"
			color="blue"
			size="sm"
			on:click={async () => {
				await SettingService.setGlobal({
					key: CUSTOM_TAGS_SETTING,
					requestBody: {
						value: [...(customTags ?? []), newTag.trim().replaceAll(' ', '_')]
					}
				})
				dispatch('refresh')
				loadCustomTags()
				sendUserToast('Tag added')
			}}
			disabled={newTag.trim() == '' || !($superadmin || $devopsRole)}
		>
			Add {#if !($superadmin || $devopsRole)}
				<span class="text-2xs text-tertiary">superadmin or devops only</span>
			{/if}
		</Button>
		<span class="text-sm text-primary"
			>Configure <a href="{base}/workers" target="_blank" class="inline-flex gap-1 items-baseline"
				>worker groups <ExternalLink size={12} /></a
			> to listen to tags</span
		>
		<span class="text-2xs text-tertiary"
			>For tags specific to some workspaces, use <pre class="inline">tag(workspace1+workspace2)</pre
			></span
		>
		<span class="text-2xs text-tertiary"
			>To exclude 'workspace1' and 'workspace2' from a tag, use <pre
				class="inline">tag(^workspace1^workspace2)</pre
			></span
		>
		<span class="text-2xs text-tertiary"
			>For dynamic tags based on the workspace, use <pre class="inline">$workspace</pre>, e.g:
			<pre class="inline">tag-$workspace</pre></span
		>
	{/if}
</div>
