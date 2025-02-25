<script lang="ts">
	import { Button } from './common'
	import Popover from './meltComponents/Popover.svelte'
	import { autoPlacement } from '@floating-ui/core'
	import ChangeInstanceUsernameInner from './ChangeInstanceUsernameInner.svelte'

	export let email: string
	export let username: string
	export let isConflict = false
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
	<svelte:fragment slot="trigger">
		<Button color={isConflict ? 'red' : 'light'} size="xs" spacingSize="xs2" nonCaptureEvent={true}
			>{isConflict ? 'Fix username conflict' : 'Change username'}</Button
		>
	</svelte:fragment>
	<svelte:fragment slot="content">
		<ChangeInstanceUsernameInner
			{email}
			{username}
			{isConflict}
			on:close={() => close()}
			on:renamed
		/>
	</svelte:fragment>
</Popover>
