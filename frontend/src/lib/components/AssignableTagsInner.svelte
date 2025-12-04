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
	import TextInput from './text_input/TextInput.svelte'
	import { twMerge } from 'tailwind-merge'
	import Badge from './common/badge/Badge.svelte'

	interface Props {
		variant?: 'popover' | 'drawer'
	}

	let { variant = 'popover' }: Props = $props()

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

	function onKeyDown(e: KeyboardEvent) {
		if (e.key === 'Enter' && newTag.trim() !== '' && tagEditor) {
			e.stopPropagation()
			e.preventDefault()
			saveCustomTag(newTag)
		}
	}

	async function saveCustomTag(tag: string, restoreCustomTags: boolean = false) {
		try {
			await SettingService.setGlobal({
				key: CUSTOM_TAGS_SETTING,
				requestBody: { value: [...(customTags ?? []), tag.trim().replaceAll(' ', '_')] }
			})
			dispatch('refresh')
			loadCustomTags()
			sendUserToast(restoreCustomTags ? 'Tag restored' : 'Tag added')
			if (!restoreCustomTags) {
				newTag = ''
			}
		} catch (err) {
			sendUserToast(`Could not ${restoreCustomTags ? 'restore' : 'save'} custom tag: ${err}`, true)
		}
	}
</script>

<svelte:window onkeydown={onKeyDown} />

<div
	class="flex flex-col gap-2"
	class:w-72={variant === 'popover'}
	class:p-4={variant === 'popover'}
>
	{#if customTags == undefined}
		<Loader2 class="animate-spin" />
	{:else}
		<div class="flex flex-row flex-wrap gap-y-1 gap-x-2">
			{#each customTags as customTag}
				<Badge color="blue">
					{customTag}

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
									sendUserToast('Tag removed', false, [
										{
											label: 'Undo',
											callback: () => {
												saveCustomTag(customTag, true)
											}
										}
									])
								})
							)}
						>
							<X size={12} />
						</button>
					{/if}
					<NoWorkerWithTagWarning tag={customTag} />
				</Badge>
			{/each}
		</div>

		<div class={twMerge('w-full flex gap-2', variant === 'popover' ? 'flex-col ' : 'flex-row ')}>
			<TextInput bind:value={newTag} />
			<Button
				variant="accent"
				unifiedSize="md"
				onClick={() => saveCustomTag(newTag)}
				disabled={newTag.trim() == '' || !tagEditor}
				wrapperClasses="min-w-24"
			>
				Add custom tag {#if !tagEditor}
					<span class="text-2xs text-primary">superadmin or devops only</span>
				{/if}
			</Button>
		</div>
		{#if extractedCustomTag}
			<div class="text-2xs text-primary p-2 bg-surface-secondary rounded">
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

		<span class="text-2xs text-secondary leading-relaxed">
			{#if variant !== 'drawer'}
				Configure <a
					href="https://www.windmill.dev/docs/core_concepts/worker_groups"
					target="_blank"
					class="inline-flex gap-1 items-baseline"
					>worker groups <ExternalLink size={12} />
				</a>
				to listen to tags.
				<br />
			{/if}

			For tags specific to some workspaces, use
			<pre class="inline text-emphasis">tag(workspace1+workspace2)</pre>
			<br />{#if variant !== 'drawer'}<br />{/if}
			To exclude 'workspace1' and 'workspace2' from a tag, use
			<pre class="inline text-emphasis">tag(^workspace1^workspace2)</pre>
			<br />{#if variant !== 'drawer'}<br />{/if}
			For
			<a
				href="https://www.windmill.dev/docs/core_concepts/worker_groups#dynamic-tag"
				target="_blank">dynamic tags <ExternalLink size={12} class="inline-block" /></a
			>
			based on the workspace, use <pre class="inline text-emphasis">$workspace</pre>, e.g:
			<pre class="inline text-emphasis">tag-$workspace</pre><br />
			{#if variant !== 'drawer'}<br />{/if}

			For
			<a
				href="https://www.windmill.dev/docs/core_concepts/worker_groups#dynamic-tag"
				target="_blank">dynamic tags <ExternalLink size={12} class="inline-block" /></a
			>
			based on args input, use <pre class="inline text-emphasis">$args[a.b.c]</pre> where
			<pre class="inline">a.b.c</pre> is the path to the value in the args object.
		</span>
	{/if}
</div>
