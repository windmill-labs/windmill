<script lang="ts">
	import { Button, type ButtonType } from '$lib/components/common'
	import GoogleIcon from '$lib/components/icons/GoogleIcon.svelte'

	interface Props {
		disabled?: boolean
		unifiedSize?: ButtonType.UnifiedSize
		onClick?: ((e?: Event) => void) | undefined
	}

	let { disabled = false, unifiedSize = 'md', onClick = undefined }: Props = $props()
</script>

<!-- developers.google.com/identity/branding-guidelines requires Google's own branding on the
     control that starts a Google sign-in, and allows a custom button for it as long as the G
     mark is unaltered and the label is one of theirs. So this is a design-system Button rather
     than the PNG Google ships, which carried no hover, focus or disabled state and stood at a
     different height than the buttons beside it. -->
<Button variant="default" {unifiedSize} {disabled} startIcon={{ icon: GoogleIcon }} {onClick}>
	Sign in with Google
</Button>
