<script lang="ts">
	import Popover from '$lib/components/Popover.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { classNames } from '$lib/utils'
	import { Bug } from 'lucide-svelte'

	interface Props {
		hasError?: boolean
		children?: import('svelte').Snippet
	}

	let { hasError = false, children }: Props = $props()
</script>

{#if hasError}
	<div class={classNames('bg-red-100 w-full h-full flex items-center justify-center text-red-500')}>
		<Popover notClickable placement="bottom" popupClass="!bg-surface border w-96">
			<Bug size={14} />
			{#snippet text()}
				<span>
					<div class="bg-surface">
						<Alert type="error" title="Error during execution">
							<div class="flex flex-col gap-2">
								One of the configuration of the component is invalid. Please check the configuration
								and try again.
							</div>
						</Alert>
					</div>
				</span>
			{/snippet}
		</Popover>
	</div>
{:else}
	{@render children?.()}
{/if}
