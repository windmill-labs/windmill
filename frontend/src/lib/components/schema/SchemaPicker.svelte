<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import CustomPopover from '$lib/components/CustomPopover.svelte'
	import SchemaViewer from '$lib/components/SchemaViewer.svelte'
	import { Info, Trash2 } from 'lucide-svelte'
	import { copyToClipboard } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import { convert } from '@redocly/json-to-json-schema'
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import { twMerge } from 'tailwind-merge'

	export let payloadData: Record<string, any>
	export let testKind: 'main' | 'preprocessor' = 'main'
	export let isFlow = false
	export let canEdit = false
	export let deleteLoading = false
	export let hasPreprocessor = false
	export let allowApplyArgs = true
	export let date: string | undefined
	export let selected = false

	const schema =
		isFlow && testKind === 'main' ? { required: [], properties: {}, ...convert(payloadData) } : {}

	const dispatch = createEventDispatcher()

	let hovering = false
	function formatDate(dateString: string | undefined): string {
		if (!dateString) return ''
		const date = new Date(dateString)
		return new Intl.DateTimeFormat('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		}).format(date)
	}
</script>

<button
	class={twMerge(
		'w-full rounded-sm px-2',
		hovering ? 'bg-surface-hover' : '',
		selected ? 'bg-surface-selected' : ''
	)}
	on:click={() => {
		dispatch('select')
	}}
	on:mouseenter={() => {
		hovering = true
	}}
	on:mouseleave={() => {
		hovering = false
	}}
>
	<div class="flex flex-row gap-2">
		<slot name="start" />

		<div
			class="text-2xs font-normal text-left p-2 rounded-md overflow-auto grow-0 text-ellipsis whitespace-nowrap scrollbar-none"
			title={formatDate(date)}
		>
			{formatDate(date)}
		</div>

		<CustomPopover class="grow min-w-12 ">
			<!-- svelte-ignore a11y-click-events-have-key-events -->
			<!-- svelte-ignore a11y-no-static-element-interactions -->
			<div
				class={twMerge(
					'text-xs border font-normal text-left p-1 rounded-md overflow-auto grow whitespace-nowrap scrollbar-none',
					hovering && 'border-surface'
				)}
				on:click={() => {
					copyToClipboard(JSON.stringify(payloadData))
				}}
				on:mouseenter={() => {
					hovering = true
				}}
				on:mouseleave={() => {
					hovering = false
				}}
			>
				{JSON.stringify(payloadData)}
			</div>
			<svelte:fragment slot="overlay">
				<div class="min-w-[400px]">
					<ObjectViewer json={payloadData} />
				</div>
			</svelte:fragment>
		</CustomPopover>

		{#if isFlow && testKind === 'main' && false}
			<CustomPopover>
				<Button
					size="xs2"
					color={allowApplyArgs ? 'light' : 'dark'}
					variant={'border'}
					on:click={() => {
						dispatch('updateSchema', payloadData)
					}}
					wrapperClasses="h-full"
				>
					Apply schema
				</Button>

				<svelte:fragment slot="overlay">
					{#if schema}
						<div class="min-w-[400px]">
							<SchemaViewer {schema} />
						</div>
					{/if}
				</svelte:fragment>
			</CustomPopover>
		{/if}

		{#if testKind === 'preprocessor' && !hasPreprocessor && allowApplyArgs}
			<CustomPopover noPadding>
				<Button
					size="xs"
					color="dark"
					disabled
					endIcon={{
						icon: Info
					}}
					wrapperClasses="h-full"
				>
					Apply args
				</Button>
				<svelte:fragment slot="overlay">
					<div class="text-sm p-2 flex flex-col gap-1 items-start">
						<p>You need to add a preprocessor to use preprocessor captures as args</p>
						<Button
							size="xs"
							color="dark"
							on:click={() => {
								dispatch('addPreprocessor')
							}}
						>
							Add preprocessor
						</Button>
					</div>
				</svelte:fragment>
			</CustomPopover>
		{:else if allowApplyArgs}
			<Button
				size="xs"
				color="dark"
				on:click={() => {
					if (isFlow && testKind === 'main') {
						dispatch('updateSchema', { schema, redirect: false })
					}
					dispatch('applyArgs', {
						kind: testKind,
						args: payloadData
					})
				}}
				disabled={testKind === 'preprocessor' && !hasPreprocessor}
			>
				{isFlow && testKind === 'main' ? 'Apply schema and args' : 'Apply args'}
			</Button>
		{/if}

		{#if canEdit}
			<Button
				size="xs2"
				color="red"
				iconOnly
				startIcon={{ icon: Trash2 }}
				loading={deleteLoading}
				on:click={() => {
					dispatch('delete')
				}}
			/>
		{/if}

		{#if hovering}
			<slot name="extra" />
		{/if}
	</div>
</button>

<style>
	.scrollbar-none {
		-ms-overflow-style: none; /* IE and Edge */
		scrollbar-width: none; /* Firefox */
	}

	.scrollbar-none::-webkit-scrollbar {
		display: none; /* Chrome, Safari and Opera */
	}
</style>
