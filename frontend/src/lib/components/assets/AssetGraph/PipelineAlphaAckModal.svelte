<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import { AlertTriangle, ArrowRight } from 'lucide-svelte'

	interface Props {
		open: boolean
		onAck: () => void
	}
	let { open = $bindable(), onAck }: Props = $props()
</script>

<Modal bind:open title="Pipelines is in alpha" kind="X">
	<div class="flex flex-col gap-4">
		<div class="flex items-start gap-3">
			<div
				class="shrink-0 flex h-10 w-10 items-center justify-center rounded-full bg-amber-100 dark:bg-amber-800/50"
			>
				<AlertTriangle size={20} class="text-amber-500 dark:text-amber-400" />
			</div>
			<div class="flex flex-col gap-2 text-sm text-secondary">
				<p>
					Pipelines is an <span class="font-semibold text-emphasis">alpha feature preview</span>.
					The API, on-disk format, and UI are still evolving and may change without notice.
				</p>
				<p>
					Pipelines built today may need migration as the feature stabilizes. Avoid relying on them
					for production workloads until they leave alpha.
				</p>
			</div>
		</div>
		<div class="flex items-center justify-end gap-2 pt-2">
			<Button variant="accent" unifiedSize="sm" onclick={onAck} endIcon={{ icon: ArrowRight }}>
				I understand, continue
			</Button>
		</div>
	</div>
</Modal>
