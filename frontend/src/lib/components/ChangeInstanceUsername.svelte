<script lang="ts">
	import { Button } from './common'
	import Popover from './meltComponents/Popover.svelte'
	import { autoPlacement } from '@floating-ui/core'
	import ChangeInstanceUsernameInner from './ChangeInstanceUsernameInner.svelte'
	import { AlertTriangle } from 'lucide-svelte'

	interface Props {
		email: string
		username: string
		isConflict?: boolean
	}

	let { email, username, isConflict = false }: Props = $props()
</script>

<Popover
	floatingConfig={{
		middleware: [
			autoPlacement({
				allowedPlacements: ['bottom-end', 'top-end']
			})
		]
	}}
	closeButton
>
	{#snippet trigger()}
		{#if isConflict}
			<!-- An icon rather than a labelled button: the username column truncates text but
			     cannot truncate a button, so a wide trigger here forced the whole table to scroll. -->
			<Button
				variant="subtle"
				unifiedSize="xs"
				iconOnly
				startIcon={{ icon: AlertTriangle }}
				btnClasses="text-yellow-600 dark:text-yellow-400"
				title="No instance username. Click to fix the conflict."
				aria-label="Fix username conflict"
				nonCaptureEvent={true}
			/>
		{:else}
			<Button variant="default" unifiedSize="xs" nonCaptureEvent={true}>Change username</Button>
		{/if}
	{/snippet}
	{#snippet content()}
		<ChangeInstanceUsernameInner
			{email}
			{username}
			{isConflict}
			on:close={() => close()}
			on:renamed
		/>
	{/snippet}
</Popover>
