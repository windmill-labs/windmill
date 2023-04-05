<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../types'
	import { secondaryMenu } from './settingsPanel/secondaryMenu'

	const { connectingInput } = getContext<AppViewerContext>('AppViewerContext')

	function stopConnecting() {
		$connectingInput.opened = false
		$connectingInput.input = undefined

		secondaryMenu.close()
	}
</script>

<Alert title="Connecting" type="info">
	<div class="flex gap-2 flex-col">
		Click on the output of the component you want to connect to on the left panel.
		<div class="my-2">
			<ol class="list-disc flex gap-2 flex-col">
				<li>
					You can also navigate your app in a viewer mode: You won't be able to fire any events.
				</li>
				<li> Clicking on a component will open the component's output on the left. </li>
			</ol>
		</div>
		<div>
			<Button color="blue" variant="border" size="xs" on:click={stopConnecting}>
				Stop connecting
			</Button>
		</div>
	</div>
</Alert>
