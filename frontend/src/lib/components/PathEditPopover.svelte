<!--
@component
The editor behind an item's name: clicking the name it is given opens the summary and the path.
A caller that has no name to give it — a bar with a summary field of its own — gets a pen instead.

A caller that draws the path itself should render it from `snapshotPath ?? path`: while the
popover is open this holds the path as it was when it opened, so the trail — and the popover
anchored to it — does not reflow under the pointer as the user types, which floating-ui would
follow.
-->
<script lang="ts">
	import type { ComponentProps, Snippet } from 'svelte'
	import { Alert } from '$lib/components/common'
	import PathEditPen from '$lib/components/PathEditPen.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Path from '$lib/components/Path.svelte'
	import Label from '$lib/components/Label.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { isOwner } from '$lib/utils'
	import { userStore } from '$lib/stores'
	import {
		useOperatingUser,
		useOperatingWorkspace
	} from '$lib/components/operatingWorkspace.svelte'

	const operatingWorkspace = useOperatingWorkspace()
	const operatingUser = useOperatingUser()
	const actingUser = $derived(operatingUser.current)

	interface Props {
		/** The item's human name. Committed trimmed, like the bar's own field did. */
		summary?: string
		/** When false the summary is shown but not editable, for a host whose `customUi` says so. */
		summaryEditable?: boolean
		/** When false the path is shown but not editable, the same way. A host may allow one and
		 *  refuse the other, and the popover offers whichever it is given. */
		pathEditable?: boolean
		path?: string
		/** The item's *saved* path on the server, so the popover can say a rename needs deploying. */
		savedPath?: string
		/** Scopes the path picker's validation, and names the item in the placeholder and the
		 *  deploy note. An agent lives at a resource path, so this is wider than the three
		 *  workspace item kinds. */
		kind?: ComponentProps<typeof Path>['kind']
		/** Workspace the path picker scopes to; defaults to the operating workspace. */
		workspaceId?: string
		/** When set, warns that a redeploy happens on behalf of the current user instead. */
		onBehalfOfEmail?: string | undefined
		/** Only read when there is no `label`, where the pen is the whole trigger. */
		penVisibility?: 'hover' | 'always'
		/** The path as it was when the popover opened, undefined while it is closed. See the
		 *  component note: the caller draws its trail from this so it holds still mid-rename. */
		snapshotPath?: string | undefined
		/** The item's name, rendered inside the trigger so clicking it opens the editor. Without
		 *  one the trigger is a pen, for a host that draws its name somewhere this cannot reach. */
		label?: Snippet
		/** The trigger's own classes, for a host that needs it to wear the look of the row it sits
		 *  in — the band's path segment. The default is a name with a hover fill behind it: the
		 *  click opens an editor here rather than going anywhere, and the fill is what says the
		 *  name is a control at all now that no pen sits beside it. */
		triggerClass?: string
		/** Bindable, for a host that drives the popover from elsewhere. */
		open?: boolean
		/** Which field takes the cursor on open. A popover anchored under the path opens on the
		 *  path, one under the summary opens on the summary: the click says which half of the name
		 *  the user came to change. `Path` autofocuses itself, so 'path' is the absence of a
		 *  claim on the summary rather than a claim of its own. */
		focusField?: 'summary' | 'path'
		/** The path field's verdict, for a host that refuses to deploy while it is non-empty.
		 *  A host whose item has a second path field has to bind both to the same slot, or
		 *  whichever one is unmounted leaves its last verdict standing. */
		error?: string
	}

	let {
		summary = $bindable(),
		summaryEditable = true,
		pathEditable = true,
		path = $bindable(),
		savedPath,
		kind = 'flow',
		workspaceId,
		onBehalfOfEmail,
		penVisibility = 'hover',
		label,
		triggerClass = 'inline-flex items-center gap-1.5 min-w-0 max-w-full px-1 py-0.5 rounded cursor-pointer text-left hover:bg-surface-hover transition-colors',
		open = $bindable(false),
		focusField = 'summary',
		snapshotPath = $bindable(),
		error = $bindable()
	}: Props = $props()

	const penLabel = $derived(
		summaryEditable && pathEditable
			? 'Edit summary and path'
			: summaryEditable
				? 'Edit summary'
				: 'Edit path'
	)

	// Treat an empty path as ownable so the popover lets a user pick the path for a brand-new
	// item. `Path.reset()` then synthesizes a default under their own user/folder scope.
	const own = $derived(pathEditable && (!path || isOwner(path, actingUser, $operatingWorkspace)))

	function setOpen(v: boolean) {
		open = v
		// Only snapshot when there is a path to freeze. A new flow or script opens with `path === ''`
		// and relies on `Path.reset()` to seed one; snapshotting the empty value would leave the
		// trail blank for the popover's whole lifetime.
		snapshotPath = v && path ? path : undefined
	}
</script>

<!-- `openFocus`: `#path` is the name field inside `Path`. Without naming it melt lands on the
     row's first button, so a popover opened from the path would not be typing into the path. -->
<Popover
	placement="bottom-start"
	contentClasses="p-4"
	class={triggerClass}
	triggerAttrs={{ title: penLabel, 'aria-label': penLabel }}
	usePointerDownOutside
	excludeSelectors=".drawer"
	disableFocusTrap
	closeOnOtherPopoverOpen
	openFocus={focusField === 'path'
		? '#path'
		: summaryEditable
			? '[data-path-edit-summary]'
			: undefined}
	bind:isOpen={() => open, setOpen}
>
	{#snippet trigger()}
		{#if label}
			<!-- The name is the affordance; a pen next to it would be a second one for the same
			     click, and the band keeps its line quiet. -->
			{@render label()}
		{:else}
			<PathEditPen label={penLabel} visibility={penVisibility} {open} />
		{/if}
	{/snippet}
	{#snippet content()}
		<div class="flex flex-col gap-6 w-[480px]">
			<!-- The summary first: it is the name a reader sees, and the path below is where that
			     name lives. -->
			{#if summaryEditable}
				<Label label="Summary">
					<TextInput
						bind:value={summary}
						inputProps={{
							placeholder: 'Add a summary...',
							'data-path-edit-summary': '',
							// The popover sits in a page that binds keys of its own (the editors all do),
							// and a summary is prose — it must not reach them. Enter commits the line and
							// closes, which is what a one-line field reads as; the path below is reached
							// with Tab, not Enter.
							onkeydown: (e: KeyboardEvent) => {
								e.stopPropagation()
								if (e.key === 'Enter') {
									e.preventDefault()
									summary = summary?.trim()
									setOpen(false)
								}
							},
							onblur: () => (summary = summary?.trim())
						}}
						size="sm"
					/>
				</Label>
			{:else if summary}
				<Label label="Summary">
					<span class="text-xs text-secondary">{summary}</span>
				</Label>
			{/if}
			{#if own}
				<!-- Both of these say the same thing — the item's own saved path is not a collision —
				     and both are needed, because `Path` skips the whole existence check while the
				     typed path equals `initialPath`. Seeding that from the working path instead
				     would make a rename to a taken path look untaken the moment the popover is
				     reopened on it. With no saved path there is nothing the item occupies, so every
				     path is checked. -->
				<Path
					autofocus
					bind:path
					bind:error
					initialPath={savedPath ?? path ?? ''}
					allowedExistingPath={savedPath}
					checkInitialPathExistence={savedPath == undefined}
					namePlaceholder={kind}
					{kind}
					size="sm"
					drawerOffset={4000}
					workspaceOverride={workspaceId}
				/>
				{#if savedPath && path && path !== savedPath}
					<Alert
						type="info"
						size="xs"
						title="Deploy the {kind} to make the path change effective."
					/>
				{/if}
				{#if onBehalfOfEmail}
					<Alert type="info" title="Run on behalf of" size="xs">
						This flow will be redeployed on behalf of you ({$userStore?.email}) instead of {onBehalfOfEmail}
					</Alert>
				{/if}
			{:else}
				<Label label="Path">
					<span class="text-xs font-mono text-secondary">{path}</span>
					<p class="text-2xs text-tertiary mt-1">
						{pathEditable
							? 'Only the owner can change the path'
							: 'This path cannot be changed here'}
					</p>
				</Label>
			{/if}
		</div>
	{/snippet}
</Popover>
