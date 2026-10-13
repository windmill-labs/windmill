<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import { Button } from '$lib/components/common'
	import { Database, UserRound } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { autoPlacement } from '@floating-ui/core'
	import { enterpriseLicense } from '$lib/stores'
	import { isCloudHosted } from '$lib/cloud'

	interface Props {
		datatable_as_viewer: boolean | undefined
	}

	let { datatable_as_viewer = $bindable() }: Props = $props()

	// The setting only changes anything for a data table under roles, which needs both.
	const unavailable = $derived(
		!$enterpriseLicense
			? 'Data table roles are an Enterprise Edition feature.'
			: isCloudHosted()
				? 'Data table roles are only available on self-hosted instances.'
				: undefined
	)
</script>

<Popover
	floatingConfig={{
		middleware: [
			autoPlacement({
				allowedPlacements: ['bottom-start', 'bottom-end', 'top-start', 'top-end', 'top', 'bottom']
			})
		]
	}}
	closeButton
	contentClasses="block text-primary p-4 max-w-sm"
>
	{#snippet trigger()}
		<Button
			nonCaptureEvent={true}
			variant={datatable_as_viewer ? 'accent-secondary' : 'default'}
			unifiedSize="sm"
			startIcon={{ icon: datatable_as_viewer ? UserRound : Database }}
			btnClasses="whitespace-nowrap"
			title={datatable_as_viewer
				? "Data tables: connects with the viewer's roles"
				: "Data tables: connects with the app identity's roles"}
		>
			{datatable_as_viewer ? 'as viewer' : 'as app'}{unavailable ? ' (EE)' : ''}
		</Button>
	{/snippet}
	{#snippet content()}
		<Toggle
			checked={datatable_as_viewer ?? false}
			disabled={unavailable !== undefined && !datatable_as_viewer}
			eeOnly
			on:change={() => {
				datatable_as_viewer = datatable_as_viewer ? undefined : true
			}}
			options={{
				right: 'Connect to data tables as the viewer',
				rightTooltip:
					'Data table roles (Enterprise Edition, self-hosted) map who may use a data table to Postgres roles, so grants and row-level security policies decide what each connection sees.',
				rightDocumentationLink:
					'https://www.windmill.dev/docs/core_concepts/persistent_storage/data_tables'
			}}
		/>
		<div class="mt-3 flex flex-col gap-2 text-xs text-secondary">
			{#if unavailable}
				<p class="text-primary">{unavailable}</p>
			{/if}
			<p>
				On a data table under roles, a connection that names no role holds the roles of the identity
				the app runs as, the publisher in publisher mode, for every viewer.
			</p>
			<p>
				Turn this on to hold the roles of the person viewing the app instead, so row-level security
				answers per viewer. Anonymous viewers are refused. A connection that names a role is that
				role alone either way.
			</p>
			<p>
				Neither applies while the data table's default role is admin: such a connection is admin,
				which bypasses row-level security. Change or remove the default role in the data table's
				roles first.
			</p>
		</div>
	{/snippet}
</Popover>
