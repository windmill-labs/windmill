<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import Checkbox from '../common/checkbox/Checkbox.svelte'
	import CopyableCodeBlock from '../details/CopyableCodeBlock.svelte'
	import { plaintext } from 'svelte-highlight/languages'
	import { download } from '$lib/utils'

	interface Props {
		codes: string[]
		saved: boolean
	}

	let { codes, saved = $bindable() }: Props = $props()
</script>

<div class="flex flex-col gap-1">
	<span class="text-xs text-secondary font-normal">
		Each code logs you in once if you lose your authenticator.
	</span>
	<CopyableCodeBlock code={codes.join('\n')} language={plaintext} />
	<div class="flex">
		<Button
			unifiedSize="sm"
			variant="default"
			onClick={() => download('windmill-recovery-codes.txt', codes.join('\n'))}
		>
			Download
		</Button>
	</div>
</div>
<Alert type="warning" title="Save these codes now" size="xs">
	They are shown only once. Keep them somewhere other than this device.
</Alert>
<label class="flex flex-row gap-2 items-center text-xs text-primary font-normal">
	<Checkbox checked={saved} onChange={() => (saved = !saved)} />
	I saved my recovery codes
</label>
