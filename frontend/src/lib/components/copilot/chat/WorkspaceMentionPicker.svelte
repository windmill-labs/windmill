<!--
@component
Workspace-only picker for the AI session composer's `+ -> Mention file`
submenu. It mirrors ChatContextPicker's workspace branch, but starts directly
at the workspace root because the parent menu item already names the scope.
-->
<script lang="ts">
	import { Folder } from 'lucide-svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import DrillPicker from '$lib/components/DrillPicker.svelte'
	import type { DrillBranch, DrillLeaf, DrillNode } from '$lib/components/drillPicker'
	import {
		workspaceItemDisplayPath,
		type WorkspaceItem,
		type WorkspaceItemKind
	} from '$lib/components/workspacePicker'
	import { useWorkspaceItemsLoader } from '$lib/components/workspaceItemsLoader.svelte'
	import { buildWorkspaceTree, relativizeWorkspacePath } from '$lib/components/workspaceTree'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	import type {
		ContextElement,
		WorkspaceAppElement,
		WorkspaceFlowElement,
		WorkspaceScriptElement
	} from './context'

	interface Props {
		onSelect: (element: ContextElement) => void
		onAfterClose?: () => void
		close: (afterClose?: () => void) => void
	}

	type DrillPickerHandle = {
		handleKeydown: (e: KeyboardEvent) => void
	}

	let { onSelect, onAfterClose, close }: Props = $props()
	let picker: DrillPickerHandle | undefined = $state()

	const operatingWorkspace = useOperatingWorkspace()
	const WORKSPACE_KINDS: WorkspaceItemKind[] = ['flow', 'script', 'app']
	const loader = useWorkspaceItemsLoader(
		() => $operatingWorkspace,
		() => WORKSPACE_KINDS
	)

	const loadedForTree = $derived.by(() => {
		const loaded = loader.loaded
		if (!loaded.app) return loaded
		return { ...loaded, app: loaded.app.filter((a) => a.raw_app) }
	})

	const tree = $derived<DrillNode<WorkspaceItem>[]>(
		buildWorkspaceTree({
			loaded: loadedForTree,
			kinds: WORKSPACE_KINDS,
			loadingKind: loader.loadingKind,
			layout: 'flat'
		}) as DrillNode<WorkspaceItem>[]
	)

	function workspaceElement(item: WorkspaceItem): ContextElement | undefined {
		if (item.kind === 'script') {
			return {
				type: 'workspace_script',
				path: item.path,
				title: item.path,
				summary: item.summary,
				deletable: true
			} satisfies WorkspaceScriptElement & { deletable: boolean }
		}
		if (item.kind === 'flow') {
			return {
				type: 'workspace_flow',
				path: item.path,
				title: item.path,
				summary: item.summary,
				deletable: true
			} satisfies WorkspaceFlowElement & { deletable: boolean }
		}
		if (item.kind === 'app') {
			return {
				type: 'workspace_app',
				path: item.path,
				title: item.path,
				summary: item.summary,
				deletable: true
			} satisfies WorkspaceAppElement & { deletable: boolean }
		}
	}

	function handlePick(leaf: DrillLeaf<WorkspaceItem>) {
		const element = workspaceElement(leaf.data)
		if (!element) return
		onSelect(element)
		close(onAfterClose)
	}

	function handleScopeChange(_scope: string[]) {
		loader.ensureAll()
	}

	const pickerKeys = new Set([
		'ArrowDown',
		'ArrowUp',
		'ArrowLeft',
		'ArrowRight',
		'Backspace',
		'Enter',
		'Tab',
		'Home',
		'End'
	])

	function handleKeydownCapture(e: KeyboardEvent) {
		if (!pickerKeys.has(e.key)) return
		e.stopImmediatePropagation()
		picker?.handleKeydown(e)
	}
</script>

{#snippet leafIcon(leaf: DrillLeaf<WorkspaceItem>)}
	<RowIcon kind={leaf.data.kind} size={12} />
{/snippet}

{#snippet branchIcon(branch: DrillBranch<WorkspaceItem>)}
	{#if branch.icon}
		{@const Icon = branch.icon}
		<Icon size={12} class="shrink-0 text-tertiary" />
	{:else}
		<Folder size={12} class="shrink-0 text-tertiary" />
	{/if}
{/snippet}

<!-- Capture picker-owned keys before Melt's submenu listener handles menu movement. -->
<div onkeydowncapture={handleKeydownCapture}>
	<DrillPicker
		bind:this={picker}
		{tree}
		onPick={handlePick}
		{leafIcon}
		{branchIcon}
		leafSecondary={(leaf, scope) => relativizeWorkspacePath(workspaceItemDisplayPath(leaf.data), scope)}
		onScopeChange={handleScopeChange}
		onFilterChange={loader.onFilterChange}
		rootLoading={WORKSPACE_KINDS.some((k) => !loader.loaded[k] && loader.loadingKind[k])}
	/>
</div>
