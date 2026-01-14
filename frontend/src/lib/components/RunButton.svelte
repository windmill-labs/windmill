<script lang="ts">
	import { Button, ButtonType } from '$lib/components/common'
	import { CornerDownLeft, Loader2 } from 'lucide-svelte'

	interface Props {
		isLoading: any
		hideShortcut?: boolean
		onRun: () => Promise<void>
		onCancel: () => Promise<void>
		size?: ButtonType.UnifiedSize
	}

	let { isLoading, hideShortcut = false, onRun, onCancel, size = 'sm' }: Props = $props()
</script>

{#if !isLoading}
	<Button
		loading={isLoading}
		unifiedSize={size}
		variant="accent"
		onClick={() => onRun()}
		shortCut={{ Icon: CornerDownLeft, hide: hideShortcut }}
	>
		Run
	</Button>
{:else}
	<Button unifiedSize={size} variant="accent" destructive onClick={() => onCancel()}>
		<Loader2 size={14} class="animate-spin mr-2" />
		Cancel
	</Button>
{/if}
