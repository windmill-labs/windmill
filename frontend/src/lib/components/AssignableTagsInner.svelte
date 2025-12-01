<script lang="ts">
	import { preventDefault, stopPropagation } from 'svelte/legacy'

	import { Button } from './common'
	import { ExternalLink, Loader2, X } from 'lucide-svelte'
	import { SettingService, WorkerService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { superadmin, devopsRole } from '$lib/stores'
	import NoWorkerWithTagWarning from './runs/NoWorkerWithTagWarning.svelte'
	import { CUSTOM_TAGS_SETTING } from '$lib/consts'
	import { createEventDispatcher } from 'svelte'

	let newTag: string = $state('')
	let customTags: string[] | undefined = $state(undefined)

	let tagEditor = $derived(Boolean($superadmin || $devopsRole))

	async function loadCustomTags() {
		try {
			customTags =
				(await WorkerService.getCustomTags({
					showWorkspaceRestriction: tagEditor
				})) ?? []
		} catch (err) {
			sendUserToast(`Could not load global cache: ${err}`, true)
		}
	}

	const dispatch = createEventDispatcher()

	const customTagRegex = /^([\w-]+)\(((?:[\w-]+\+)*[\w-]+|(?:\^[\w-]+)+)\)$/
	const dynamicTagRegex = /\$args\[((?:\w+\.)*\w+)\]/

	let dynamicTag = $derived.by(() => {
		let r = newTag.trim()
		if (r == '') return undefined
		let matched = r.match(dynamicTagRegex)
		return matched?.[1]
	})

	let extractedCustomTag = $derived.by(() => {
		let r = newTag.trim()
		if (r == '') return undefined
		let matched = r.match(customTagRegex)
		console.log(matched)
		let tag = matched?.[1]
		let workspaces_raw = matched?.[2]
		let tag_type = workspaces_raw?.includes('^') ? 'exclude' : 'include'
		if (tag_type == 'exclude') {
			workspaces_raw = workspaces_raw?.slice(1)
		}
		let workspaces = workspaces_raw?.split(tag_type == 'include' ? '+' : '^')
		if (!workspaces_raw || workspaces_raw?.length == 0) {
			return undefined
		}
		return { tag, workspaces, tag_type }
	})

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
					{#if tagEditor}
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
						</button>
					{/if}
					<NoWorkerWithTagWarning tag={customTag} />
				</div>
			{/each}
		</div>
		<input type="text" bind:value={newTag} />
		{#if extractedCustomTag}
			<div class="text-2xs text-primary p-2 bg-surface-secondary rounded border">
				<div class="font-medium mb-1">Workspace specific tag</div>
				<div>
					<b>Tag:</b>
					{extractedCustomTag.tag}
				</div>
				<div>
					<b>Workspaces:</b>
					{#if extractedCustomTag.tag_type == 'include'}
						{extractedCustomTag.workspaces?.join(', ')}
					{:else}
						All workspaces except {extractedCustomTag.workspaces?.join(', ')}
					{/if}
				</div>
			</div>
		{:else if newTag.trim()}
			{#if newTag.includes('(') || newTag.includes(')') || newTag.includes('+') || newTag.includes('^') || ((newTag.includes('.') || newTag.includes('$args[')) && !dynamicTag)}
				<div class="text-2xs text-primary p-2 bg-surface-secondary rounded border">
					<div class="font-medium mb-1 text-red-500">Invalid tag</div>
					<div>
						<b>Tag:</b>
						{newTag.trim()}
					</div>
				</div>
			{:else}
				<div class="text-2xs text-primary p-2 bg-surface-secondary rounded border">
					<div class="font-medium mb-1">
						{#if newTag.includes('$workspace') || newTag.includes('$args')}
							Dynamic tag
						{:else}
							Simple tag
						{/if}
					</div>
					<div>
						<b>Tag:</b>
						{newTag.trim()}
					</div>
					{#if newTag.includes('$workspace') && !dynamicTag}
						<div>Interpolated tag based on workspace id the job was created in </div>
					{/if}
					{#if dynamicTag}
						<div>Interpolated tag based on args input of <b>{dynamicTag}</b></div>
					{/if}
				</div>
			{/if}
		{/if}

		<Button
			variant="accent"
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
			disabled={newTag.trim() == '' || !tagEditor}
		>
			Add {#if !tagEditor}
				<span class="text-2xs text-primary">superadmin or devops only</span>
			{/if}
		</Button>
		<span class="text-xs text-primary"
			>Configure <a
				href="https://www.windmill.dev/docs/core_concepts/worker_groups"
				target="_blank"
				class="inline-flex gap-1 items-baseline">worker groups <ExternalLink size={12} /></a
			> to listen to tags</span
		>
		<span class="text-2xs text-primary"
			>For tags specific to some workspaces, use <pre class="inline">tag(workspace1+workspace2)</pre
			></span
		>
		<span class="text-2xs text-primary"
			>To exclude 'workspace1' and 'workspace2' from a tag, use <pre class="inline"
				>tag(^workspace1^workspace2)</pre
			></span
		>
		<span class="text-2xs text-primary"
			>For <a
				href="https://www.windmill.dev/docs/core_concepts/worker_groups#dynamic-tag"
				target="_blank">dynamic tags</a
			>
			based on the workspace, use <pre class="inline">$workspace</pre>, e.g:
			<pre class="inline">tag-$workspace</pre></span
		>
		<span class="text-2xs text-primary"
			>For <a
				href="https://www.windmill.dev/docs/core_concepts/worker_groups#dynamic-tag"
				target="_blank">dynamic tags</a
			>
			based on args input, use <pre class="inline">$args[a.b.c]</pre> where
			<pre class="inline">a.b.c</pre> is the path to the value in the args object</span
		>
	{/if}
</div>
