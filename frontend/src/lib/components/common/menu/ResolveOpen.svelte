<script lang="ts">
	interface Props {
		open?: boolean;
		onOpen?: () => void;
		onClose?: () => void;
	}

	let { open = false, onOpen = undefined, onClose = undefined }: Props = $props();

	$effect(() => {
		if (open) {
			onOpen?.()
		} else {
			onClose?.()
		}
	});
</script>
