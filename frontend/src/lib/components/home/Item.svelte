<script lang="ts">
	import AppRow from '../common/table/AppRow.svelte'
	import FlowRow from '../common/table/FlowRow.svelte'
	import RawAppRow from '../common/table/RawAppRow.svelte'
	import ScriptRow from '../common/table/ScriptRow.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { Alert } from '$lib/components/common'
	import DeployWorkspaceDrawer from '../DeployWorkspaceDrawer.svelte'
	import MoveDrawer from '../MoveDrawer.svelte'
	import ShareModal from '../ShareModal.svelte'
	import { createEventDispatcher } from 'svelte'
	import { ArrowBigUp } from 'lucide-svelte'

	const dispatch = createEventDispatcher()

	let deleteConfirmedCallback: (() => void) | undefined = $state(undefined)
	let shareModal: any | undefined = $state()
	let moveDrawer: any | undefined = $state()
	let deploymentDrawer: any | undefined = $state()

	let menuOpen: boolean = $state(false)
	interface Props {
		item: any
		depth?: number
		showCode: (path: string, summary: string) => void
		showEditButton?: boolean
	}

	let { item, depth = 0, showCode, showEditButton = true }: Props = $props()
</script>

{#if item.type == 'script'}
	<ScriptRow
		bind:deleteConfirmedCallback
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
		{showEditButton}
	/>
{:else if item.type == 'flow'}
	<FlowRow
		bind:deleteConfirmedCallback
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
		{showEditButton}
	/>
{:else if item.type == 'app'}
	<AppRow
		bind:deleteConfirmedCallback
		marked={item.marked}
		on:change={() => dispatch('appChanged')}
		app={item}
		{moveDrawer}
		{shareModal}
		{deploymentDrawer}
		{depth}
		bind:menuOpen
		{showEditButton}
	/>
{:else if item.type == 'raw_app'}
	<RawAppRow
		marked={item.marked}
		app={item}
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
					<span
						class="inline-flex border rounded-md p-1 bg-blue-100 border-blue-200 dark:bg-blue-800 dark:border-blue-900 text-xs"
					>
						<ArrowBigUp size={18} />
					</span>
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
