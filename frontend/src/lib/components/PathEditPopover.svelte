<!--
@component
The pen that opens an item's summary and path for editing, beside the breadcrumb that shows them.

A caller that draws the path itself should render it from `snapshotPath ?? path`: while the
popover is open this holds the path as it was when it opened, so the trail — and the pen anchored
to it — does not reflow under the pointer as the user types, which floating-ui would follow.
-->
<script lang="ts">
	import type { ComponentProps } from 'svelte'
	import { Pencil } from 'lucide-svelte'
	import { Alert, Button } from '$lib/components/common'
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
		 *  refuse the other, and the pen offers whichever it is given. */
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
		penVisibility?: 'hover' | 'always'
		/** The path as it was when the popover opened, undefined while it is closed. See the
		 *  component note: the caller draws its trail from this so it holds still mid-rename. */
		snapshotPath?: string | undefined
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
		snapshotPath = $bindable(),
		error = $bindable()
	}: Props = $props()

	let open = $state(false)
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

<Popover
	placement="bottom-start"
	contentClasses="p-4"
	usePointerDownOutside
	excludeSelectors=".drawer"
	disableFocusTrap
	closeOnOtherPopoverOpen
	bind:isOpen={() => open, setOpen}
>
	{#snippet trigger()}
		<Button
			variant="subtle"
			unifiedSize="xs"
			iconOnly
			startIcon={{ icon: Pencil }}
			title={penLabel}
			aria-label={penLabel}
			btnClasses={penVisibility === 'hover' && !open
				? 'opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100'
				: ''}
		/>
	{/snippet}
	{#snippet content()}
		<div class="flex flex-col gap-6 w-[480px]">
			<!-- The summary first: it is the name a reader sees, and the path below is where that
			     name lives. Not autofocused — the pen's own job is the path, which takes the cursor. -->
			{#if summaryEditable}
				<Label label="Summary">
					<TextInput
						bind:value={summary}
						inputProps={{
							placeholder: 'Add a summary...',
							// The popover sits in a page that binds keys of its own (the editors all do),
							// and a summary is prose — it must not reach them.
							onkeydown: (e: KeyboardEvent) => e.stopPropagation(),
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
				<!-- allowedExistingPath: the item already occupies its saved path, so typing it back
				     after a rename that has not been deployed is not a collision. `initialPath`
				     cannot say so — it is the working path, which moves with the rename. -->
				<Path
					autofocus
					bind:path
					bind:error
					initialPath={snapshotPath ?? path ?? ''}
					allowedExistingPath={savedPath}
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
