<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import { Button } from '$lib/components/common'
	import { AppWindow, UserRound } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { autoPlacement } from '@floating-ui/core'
	import { enterpriseLicense } from '$lib/stores'
	import { isCloudHosted } from '$lib/cloud'

	interface Props {
		viewer_data_access: boolean | undefined
	}

	let { viewer_data_access = $bindable() }: Props = $props()

	// It only changes what data table roles and object storage permission rules decide, both
	// Enterprise Edition.
	const unavailable = $derived(
		!$enterpriseLicense
			? 'Data table roles and object storage permission rules are Enterprise Edition features.'
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
			variant={viewer_data_access ? 'accent-secondary' : 'default'}
			unifiedSize="sm"
			startIcon={{ icon: viewer_data_access ? UserRound : AppWindow }}
			iconOnly
			title={(viewer_data_access
				? 'Reads data tables and object storage as the viewer'
				: 'Reads data tables and object storage as the app') +
				(unavailable ? ' (Enterprise Edition)' : '')}
		/>
	{/snippet}
	{#snippet content()}
		<Toggle
			checked={viewer_data_access ?? false}
			disabled={unavailable !== undefined && !viewer_data_access}
			eeOnly
			on:change={() => {
				viewer_data_access = viewer_data_access ? undefined : true
			}}
			options={{
				right: 'Read data as the viewer',
				rightTooltip:
					'Data table roles map who may use a data table to Postgres roles, so grants and row-level security decide what a connection sees. Object storage permission rules decide which paths an identity may list, read and write.',
				rightDocumentationLink:
					'https://www.windmill.dev/docs/core_concepts/persistent_storage/data_tables'
			}}
		/>
		<div class="mt-3 flex flex-col gap-2 text-xs text-secondary">
			{#if unavailable}
				<p class="text-primary">{unavailable}</p>
			{/if}
			<p>
				By default this runnable reads as the identity the app runs as, the publisher in publisher
				mode, for every viewer: a data table connection that names no role holds that identity's
				roles, and object storage permission rules are checked against it.
			</p>
			<p>
				Turn this on to read as the person viewing the app instead, so row-level security and
				per-user storage paths answer per viewer. Anonymous viewers are refused. Resources,
				variables and everything else stay the app's.
			</p>
			<p>
				A data table connection that names a role is that role alone either way, and one whose
				default role is admin stays admin, which bypasses row-level security.
				{#if isCloudHosted()}Data table roles are only available on self-hosted instances.{/if}
			</p>
		</div>
	{/snippet}
</Popover>
