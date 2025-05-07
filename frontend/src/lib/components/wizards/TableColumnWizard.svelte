<script lang="ts">
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Alert from '../common/alert/Alert.svelte'
	import Label from '../Label.svelte'
	import Toggle from '../Toggle.svelte'
	import Tooltip from '../Tooltip.svelte'
	import { offset, flip, shift } from 'svelte-floating-ui/dom'
	export let column: {
		headerName: string
		hideColumn: boolean
		type: 'text' | 'badge' | 'link'
	}
</script>

<Popover
	floatingConfig={{
		strategy: 'fixed',
		placement: 'left-end',
		middleware: [offset(8), flip(), shift()]
	}}
	closeButton
	closeOnOtherPopoverOpen
>
	<svelte:fragment slot="trigger">
		<slot name="trigger" />
	</svelte:fragment>
	<svelte:fragment slot="content">
		<div class="flex flex-col w-96 p-4 gap-4">
			<span class="text-sm mb-2 leading-6 font-semibold">
				Table Column
				<Tooltip
					documentationLink="https://www.ag-grid.com/javascript-data-grid/column-definitions/"
				>
					Column definitions are used to define columns in ag-Grid.
				</Tooltip>
			</span>

			<Label label="Header name">
				<input placeholder="header name" bind:value={column.headerName} />
			</Label>

			<Label label="Show column">
				<Toggle
					on:pointerdown={(e) => {
						e?.stopPropagation()
					}}
					options={{ right: 'Hide column' }}
					bind:checked={column.hideColumn}
					size="xs"
				/>
			</Label>

			<Label label="Type">
				<select bind:value={column.type}>
					<option value="text">Text</option>
					<option value="badge">Badge</option>
					<option value="link">Link</option>
				</select>
			</Label>

			{#if column.type === 'link'}
				<Alert type="info" title="Label" size="xs">
					They are two ways to define a link:
					<ul class="list-disc list-inside">
						<li>
							<strong>String</strong>: The string will be used as the link and the label.
						</li>
						<li>
							<strong>Object</strong>: The object must have a <code>href</code> and a
							<code>label</code> property.
						</li>
					</ul>
				</Alert>
			{/if}
		</div>
	</svelte:fragment>
</Popover>
