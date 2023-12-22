<script lang="ts">
	import AppRow from '../common/table/AppRow.svelte'
	import FlowRow from '../common/table/FlowRow.svelte'
	import RawAppRow from '../common/table/RawAppRow.svelte'
	import ScriptRow from '../common/table/ScriptRow.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { Alert, Badge } from '$lib/components/common'
	import DeployWorkspaceDrawer from '../DeployWorkspaceDrawer.svelte'
	import MoveDrawer from '../MoveDrawer.svelte'
	import ShareModal from '../ShareModal.svelte'
	import { createEventDispatcher } from 'svelte'

	export let item
	export let depth: number = 0

	const dispatch = createEventDispatcher()

	let deleteConfirmedCallback: (() => void) | undefined = undefined
	let shareModal: ShareModal
	let moveDrawer: MoveDrawer
	let deploymentDrawer: DeployWorkspaceDrawer

	let menuOpen: boolean = false
	export let showCode: (path: string, summary: string) => void
</script>

{#if item.type == 'script'}
	<ScriptRow
		bind:deleteConfirmedCallback
		starred={item.starred ?? false}
		marked={item.marked}
		on:change={() => dispatch('scriptChanged')}
		script={item}
		errorHandlerMuted={item.ws_error_handler_muted === undefined ||
		item.ws_error_handler_muted === null
			? false
			: item.ws_error_handler_muted}
		{shareModal}
		{moveDrawer}
		{deploymentDrawer}
		{depth}
		bind:menuOpen
		{showCode}
	/>
{:else if item.type == 'flow'}
	<FlowRow
		bind:deleteConfirmedCallback
		starred={item.starred ?? false}
		marked={item.marked}
		on:change={() => dispatch('flowChanged')}
		flow={item}
		errorHandlerMuted={item.ws_error_handler_muted === undefined ||
		item.ws_error_handler_muted === null
			? false
			: item.ws_error_handler_muted}
		{shareModal}
		{moveDrawer}
		{deploymentDrawer}
		{depth}
		bind:menuOpen
	/>
{:else if item.type == 'app'}
	<AppRow
		bind:deleteConfirmedCallback
		starred={item.starred ?? false}
		marked={item.marked}
		on:change={() => dispatch('appChanged')}
		app={item}
		{moveDrawer}
		{shareModal}
		{deploymentDrawer}
		{depth}
		bind:menuOpen
	/>
{:else if item.type == 'raw_app'}
	<RawAppRow
		bind:deleteConfirmedCallback
		starred={item.starred ?? false}
		marked={item.marked}
		on:change={() => dispatch('rawAppChanged')}
		app={item}
		{moveDrawer}
		{shareModal}
		{deploymentDrawer}
		{depth}
		bind:menuOpen
	/>
{/if}

{#if menuOpen}
	<ConfirmationModal
		open={Boolean(deleteConfirmedCallback)}
		title="Remove"
		confirmationText="Remove"
		on:canceled={() => {
			deleteConfirmedCallback = undefined
		}}
		on:confirmed={() => {
			if (deleteConfirmedCallback) {
				deleteConfirmedCallback()
			}
			deleteConfirmedCallback = undefined
		}}
	>
		<div class="flex flex-col w-full space-y-4">
			<span>Are you sure you want to remove it?</span>
			<Alert type="info" title="Bypass confirmation">
				<div>
					You can press
					<Badge color="dark-gray">SHIFT</Badge>
					while removing to bypass confirmation.
				</div>
			</Alert>
		</div>
	</ConfirmationModal>

	<ShareModal
		bind:this={shareModal}
		on:change={() => {
			dispatch('reload')
		}}
	/>

	<DeployWorkspaceDrawer bind:this={deploymentDrawer} />
	<MoveDrawer
		bind:this={moveDrawer}
		on:update={() => {
			dispatch('reload')
		}}
	/>
{/if}
