<script lang="ts">
	import { Button } from './common'
	import Popover from './meltComponents/Popover.svelte'
	import { autoPlacement } from '@floating-ui/core'
	import ChangeInstanceUsernameInner from './ChangeInstanceUsernameInner.svelte'

	interface Props {
		email: string
		username: string
		isConflict?: boolean
		onrenamed?: (...args: any[]) => any
	}

	let { email, username, isConflict = false,
		onrenamed = undefined }: Props = $props()
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
		<Button color={isConflict ? 'red' : 'light'} size="xs" spacingSize="xs2" nonCaptureEvent={true}
			>{isConflict ? 'Fix username conflict' : 'Change username'}</Button
		>
	{/snippet}
	{#snippet content()}
		<ChangeInstanceUsernameInner
			{email}
			{username}
			{isConflict}
			onclose={() => close()}
			onrenamed={onrenamed}
		/>
	{/snippet}
</Popover>
