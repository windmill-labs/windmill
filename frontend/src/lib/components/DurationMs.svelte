<script lang="ts">
	import { msToSec } from '$lib/utils'
	import { Badge } from './common'
	import { Hourglass } from 'lucide-svelte'

	export let duration_ms: number
</script>

<Badge large icon={{ icon: Hourglass, position: 'left' }}>
	Ran in {msToSec(duration_ms)}s
</Badge>
