<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { Trash, Save, RotateCcw } from 'lucide-svelte'
	import { type Snippet } from 'svelte'
	import Toggle from '$lib/components/Toggle.svelte'

	interface Props {
		isDraftOnly: any
		hasDraft: any
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
	}

	let {
		isDraftOnly,
		hasDraft,
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
		onUpdate
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
		{#if !isDraftOnly && !hasDraft && enabled !== undefined}
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
		{#if isDraftOnly}
			<Button
				size="xs"
				startIcon={{ icon: Trash }}
				iconOnly
				color={'light'}
				on:click={() => {
					onDelete?.()
				}}
				btnClasses="hover:bg-red-500 hover:text-white"
			/>
		{:else if hasDraft}
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
		{#if canSave && isDeployed && (isDraftOnly || hasDraft)}
			<Button
				size="xs"
				startIcon={{ icon: Save }}
				disabled={saveDisabled}
				on:click={() => {
					onUpdate?.()
				}}
				loading={isLoading}
			>
				{isDraftOnly ? 'Deploy' : 'Update'}
			</Button>
		{/if}
	</div>
{/if}
