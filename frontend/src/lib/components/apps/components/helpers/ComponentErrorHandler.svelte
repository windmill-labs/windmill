<script lang="ts">
	import Popover from '$lib/components/Popover.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { classNames } from '$lib/utils'
	import { Bug } from 'lucide-svelte'

	export let hasError: boolean = false
</script>

{#if hasError}
	<div class={classNames('bg-red-100 w-full h-full flex items-center justify-center text-red-500')}>
		<Popover notClickable placement="bottom" popupClass="!bg-white border w-96">
			<Bug size={14} />
			<span slot="text">
				<div class="bg-white">
					<Alert type="error" title="Error during execution">
						<div class="flex flex-col gap-2">
							One of the configuration of the component is invalid. Please check the configuration
							and try again.
						</div>
					</Alert>
				</div>
			</span>
		</Popover>
	</div>
{:else}
	<slot />
{/if}
