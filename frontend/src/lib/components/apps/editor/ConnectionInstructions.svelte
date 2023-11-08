<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../types'
	import { secondaryMenuRight } from './settingsPanel/secondaryMenu'
	import { Plug2 } from 'lucide-svelte'

	const { connectingInput } = getContext<AppViewerContext>('AppViewerContext')

	function stopConnecting() {
		$connectingInput.opened = false
		$connectingInput.input = undefined

		secondaryMenuRight.close()
	}
</script>

<div class="m-2">
	<Alert title="Connecting" type="info">
		<div class="flex gap-2 flex-col">
			<div>
				Click on the output of the component you want to connect to on the left panel 'Outputs' or
				in the popup by clicking the <span
					><div class="inline-flex"
						><div class="bg-red-500/90 border-red-600 px-1 py-0.5"><Plug2 size={12} /></div></div
					></span
				>
			</div>
			<div>
				<Button color="blue" variant="border" size="xs" on:click={stopConnecting}>
					Stop connecting
				</Button>
			</div>
		</div>
	</Alert>
</div>
