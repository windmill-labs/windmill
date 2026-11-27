<script lang="ts">
	import { CopyButton } from '$lib/components/common'
	import { renderSVG } from 'uqr'

	interface Props {
		setup: { secret: string; otpauth_url: string }
	}

	let { setup }: Props = $props()
</script>

<div class="flex flex-col gap-1">
	<span class="text-xs text-secondary font-normal">
		Scan this QR code with an authenticator app, or open it on this device.
	</span>
	<a
		href={setup.otpauth_url}
		aria-label="Add to an authenticator app"
		class="w-40 h-40 bg-white p-1 rounded-md"
	>
		{@html renderSVG(setup.otpauth_url)}
	</a>
	<div class="flex flex-row flex-wrap gap-1 items-center text-2xs text-secondary font-normal">
		Or enter this key manually:
		<span class="font-mono text-emphasis break-all">{setup.secret}</span>
		<CopyButton value={setup.secret} />
	</div>
</div>
