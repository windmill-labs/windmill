<script lang="ts">
	import { preventDefault, stopPropagation } from 'svelte/legacy'

	import { Button } from './common'
	import { AlertTriangle, ExternalLink, Loader2, X } from 'lucide-svelte'
	import Popover from './meltComponents/Popover.svelte'
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

	// Mirrors CUSTOM_TAG_REGEX in backend/windmill-common/src/worker.rs — keep both in sync.
	const customTagRegex = /^([\w-]+)\(((?:[\w-]+\*?\+)*[\w-]+\*?|(?:\^[\w-]+\*?)+)\)$/
	// Mirrors RE_ARG_TAG and RE_FLOW_EXPR_TAG in backend/windmill-queue/src/jobs.rs.
	const dynamicTagRegex = /\$(args|flow_expr)\[((?:\w+\.)*\w+)\]/

	function formatWorkspace(w: { id: string; includeForks: boolean }) {
		return w.includeForks ? `${w.id} (and its forks)` : w.id
	}

	let dynamicTag = $derived.by(() => {
		let r = newTag.trim()
		if (r == '') return undefined
		let matched = r.match(dynamicTagRegex)
		return matched ? { kind: matched[1], path: matched[2] } : undefined
	})

	// Mirrors custom_tag_matches in backend/windmill-common/src/worker.rs: a job's tag is judged on
	// what it resolves to, and a placeholder in a custom tag stands for any text, unless two of them
	// are tied (same kind, and the same path or one inside the other). A tied entry admits only a
	// tag written exactly like it; an untied one is a pattern fenced only by its fixed text.
	function placeholdersOf(entry: string) {
		return [...entry.matchAll(new RegExp(dynamicTagRegex.source, 'g'))].map((m) => ({
			kind: m[1],
			path: m[2],
			start: m.index,
			end: m.index + m[0].length
		}))
	}
	function isTied(entry: string) {
		const placeholders = placeholdersOf(entry)
		return placeholders.some((a, i) =>
			placeholders
				.slice(i + 1)
				.some(
					(b) =>
						a.kind == b.kind &&
						(a.path == b.path || a.path.startsWith(b.path + '.') || b.path.startsWith(a.path + '.'))
				)
		)
	}
	// With nothing fixed at its start or its end, a pattern reaches almost any tag.
	function reachesAnyTag(entry: string) {
		const placeholders = placeholdersOf(entry)
		return (
			placeholders.length > 0 &&
			!isTied(entry) &&
			placeholders[0].start == 0 &&
			placeholders[placeholders.length - 1].end == entry.length
		)
	}
	let dynamicTagTied = $derived(isTied(newTag.trim()))
	let dynamicTagReachesAnyTag = $derived(reachesAnyTag(newTag.trim()))

	let extractedCustomTag = $derived.by(() => {
		let r = newTag.trim()
		if (r == '') return undefined
		let matched = r.match(customTagRegex)
		let tag = matched?.[1]
		let workspaces_raw = matched?.[2]
		let tag_type = workspaces_raw?.includes('^') ? 'exclude' : 'include'
		if (tag_type == 'exclude') {
			workspaces_raw = workspaces_raw?.slice(1)
		}
		let workspaces = workspaces_raw?.split(tag_type == 'include' ? '+' : '^').map((w) => {
			const includeForks = w.endsWith('*')
			return { id: includeForks ? w.slice(0, -1) : w, includeForks }
		})
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

	async function setCustomTags(value: string[]) {
		await SettingService.setGlobal({
			key: CUSTOM_TAGS_SETTING,
			requestBody: { value }
		})
		// Servers answer from an in-memory tag cache that replicas only refresh on the
		// next global-settings poll, so refetching here can still return the pre-write
		// list. The value just written is authoritative.
		customTags = value
		dispatch('refresh')
	}

	async function saveCustomTag(tag: string, restoreCustomTags: boolean = false) {
		try {
			await setCustomTags([...(customTags ?? []), tag.trim().replaceAll(' ', '_')])
			sendUserToast(restoreCustomTags ? 'Tag restored' : 'Tag added')
			if (!restoreCustomTags) {
				newTag = ''
			}
		} catch (err) {
			sendUserToast(`Could not ${restoreCustomTags ? 'restore' : 'save'} custom tag: ${err}`, true)
		}
	}

	async function removeCustomTag(tag: string) {
		try {
			await setCustomTags((customTags ?? []).filter((x) => x != tag))
			sendUserToast('Tag removed', false, [
				{
					label: 'Undo',
					callback: () => {
						saveCustomTag(tag, true)
					}
				}
			])
		} catch (err) {
			sendUserToast(`Could not remove custom tag: ${err}`, true)
		}
	}
</script>

<svelte:window onkeydown={onKeyDown} />

{#snippet reachesAnyTagWarning()}
	Nothing fixed at its start or its end, so it allows anyone to reach almost any tag. We highly
	recommend a prefix or a suffix, ideally both, to limit its reach: for instance
	<code>gpu-$args[size]</code> or <code>gpu-$args[size]-eu</code>.
{/snippet}

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
					{#if reachesAnyTag(customTag)}
						<Popover
							openOnHover
							placement="top"
							class="inline-flex items-center"
							triggerAttrs={{ 'aria-label': 'Warning: this tag allows reaching almost any tag' }}
						>
							{#snippet trigger()}
								<AlertTriangle size={14} class="text-yellow-500" />
							{/snippet}
							{#snippet content()}
								<div class="max-w-72 p-3 text-xs text-primary">
									{@render reachesAnyTagWarning()}
								</div>
							{/snippet}
						</Popover>
					{/if}

					{#if tagEditor}
						<button
							class="z-10 rounded-full p-1 duration-200 hover:bg-gray-200"
							aria-label="Remove item"
							onclick={stopPropagation(preventDefault(() => removeCustomTag(customTag)))}
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
						{extractedCustomTag.workspaces?.map(formatWorkspace).join(', ')}
					{:else}
						All workspaces except {extractedCustomTag.workspaces?.map(formatWorkspace).join(', ')}
					{/if}
				</div>
			</div>
		{:else if newTag.trim()}
			{#if newTag.includes('(') || newTag.includes(')') || newTag.includes('+') || newTag.includes('^') || newTag.includes('*') || ((newTag.includes('.') || newTag.includes('$args[') || newTag.includes('$flow_expr[')) && !dynamicTag)}
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
						{#if newTag.includes('$workspace') || dynamicTag}
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
					{#if dynamicTag?.kind == 'flow_expr'}
						<div>
							Interpolated tag based on the flow value at <b>{dynamicTag.path}</b>, resolved when
							the flow step starts
						</div>
					{:else if dynamicTag}
						<div>Interpolated tag based on args input of <b>{dynamicTag.path}</b></div>
					{/if}
					{#if dynamicTagTied}
						<div class="mt-1">
							Its placeholders read the same value, or one reads inside the other, so it allows only
							jobs whose tag is written exactly like this
						</div>
					{:else if dynamicTagReachesAnyTag}
						<div class="mt-1 text-yellow-600 dark:text-yellow-500">
							{@render reachesAnyTagWarning()}
						</div>
					{:else if dynamicTag}
						<div class="mt-1">
							Allows any tag it resolves to, with the text around the placeholder as written
						</div>
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
			Forks of a workspace do not get its tags. Suffix a workspace with
			<pre class="inline text-emphasis">*</pre>
			to also cover its forks, e.g. <pre class="inline text-emphasis">tag(workspace1*)</pre>
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
			<br />{#if variant !== 'drawer'}<br />{/if}
			For
			<a
				href="https://www.windmill.dev/docs/core_concepts/worker_groups#dynamic-tag"
				target="_blank">dynamic tags <ExternalLink size={12} class="inline-block" /></a
			>
			based on flow step results, flow input or flow env, use
			<pre class="inline text-emphasis">$flow_expr[results.a.b.c]</pre> where
			<pre class="inline">a</pre> is the step id, or
			<pre class="inline text-emphasis">$flow_expr[flow_input.a.b.c]</pre> and
			<pre class="inline text-emphasis">$flow_expr[flow_env.a.b.c]</pre>.
			<br />{#if variant !== 'drawer'}<br />{/if}
			A dynamic tag is checked on the tag it resolves to: it is allowed when that tag is listed here,
			or fits a listed dynamic tag such as
			<pre class="inline text-emphasis">gpu-$args[size]</pre>, which allows any tag starting with
			<pre class="inline">gpu-</pre>.
		</span>
	{/if}
</div>
