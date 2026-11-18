<script lang="ts">
	import { Badge } from '$lib/components/common'

	interface Props {
		id: string
	}

	let { id }: Props = $props()
</script>

<!-- Styled like the id badge on a graph node, so a step named in text reads as the same thing. -->
<Badge
	color="transparent"
	class="border-none"
	wrapperClass="rounded-md bg-component-virtual-node text-emphasis align-middle"
	baseClass="!px-1"
	title={id}
>
	<span class="text-2xs font-normal">{id}</span>
</Badge>
