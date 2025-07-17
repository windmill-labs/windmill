<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { Save, RotateCcw } from 'lucide-svelte'
	import { type Snippet } from 'svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { Tooltip } from '../meltComponents'
	import DeleteTriggerButton from './DeleteTriggerButton.svelte'
	import type { Trigger } from './utils'

	interface Props {
		saveDisabled: any
		enabled: boolean | undefined
		allowDraft: any
		edit: any
		isLoading: any
		permissions: 'write' | 'create' | 'none'
		isDeployed: boolean
		extra?: Snippet
		onDelete?: () => void
		onReset?: () => void
		onToggleEnabled?: (enabled: boolean) => void
		onUpdate?: () => void
		cloudDisabled?: boolean
		trigger?: Trigger
	}

	let {
		saveDisabled,
		enabled,
		allowDraft,
		edit,
		isLoading,
		permissions,
		isDeployed,
		extra,
		onDelete,
		onReset,
		onToggleEnabled,
		onUpdate,
		cloudDisabled = false,
		trigger
	}: Props = $props()

	const canSave = $derived((permissions === 'write' && edit) || permissions === 'create')
</script>

{#if !allowDraft}
	{@render extra?.()}
	{#if edit && enabled !== undefined}
		<Toggle
			size="sm"
			disabled={permissions === 'none'}
			checked={enabled}
			options={{ right: 'enable', left: 'disable' }}
			on:change={({ detail }) => {
				onToggleEnabled?.(detail)
			}}
		/>
	{/if}
	{#if canSave}
		<Button
			size="sm"
			startIcon={{ icon: Save }}
			disabled={saveDisabled}
			on:click={() => {
				onUpdate?.()
			}}
			loading={isLoading}
		>
			Save
		</Button>
	{/if}
{:else}
	<div class="flex flex-row gap-2 items-center">
		{#if !trigger?.draftConfig && enabled !== undefined}
			<div class="center-center">
				<Toggle
					size="2sm"
					disabled={permissions === 'none'}
					checked={enabled}
					options={{ right: 'enable', left: 'disable' }}
					on:change={({ detail }) => {
						onToggleEnabled?.(detail)
					}}
				/>
			</div>
		{/if}
		{#if trigger?.isDraft || permissions === 'create'}
			<DeleteTriggerButton {onDelete} {trigger} />
		{:else if !trigger?.isDraft && trigger?.draftConfig}
			<Button
				size="xs"
				startIcon={{ icon: RotateCcw }}
				color={'light'}
				on:click={() => {
					onReset?.()
				}}
			>
				Reset changes
			</Button>
		{/if}
		{#if canSave}
			<Tooltip placement="bottom-end" disablePopup={!saveDisabled && !cloudDisabled && isDeployed}>
				<Button
					size="xs"
					startIcon={{ icon: Save }}
					disabled={saveDisabled || cloudDisabled || !isDeployed || !trigger?.draftConfig}
					on:click={() => {
						onUpdate?.()
					}}
					loading={isLoading}
				>
					{trigger?.isDraft ? 'Deploy' : 'Update'}
				</Button>
				<span slot="text">
					{#if !isDeployed}
						Deploy the runnable to enable trigger creation
					{:else if cloudDisabled}
						This trigger is disabled in the multi-tenant cloud
					{:else}
						Enter a valid config to {trigger?.isDraft ? 'deploy' : 'update'} the trigger
					{/if}
				</span>
			</Tooltip>
		{/if}
	</div>
{/if}
