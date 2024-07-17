<script lang="ts">
	export let title: string
	export let href: string
</script>

<div class="flex flex-col gap-2">
	<div class="flex flex-row items-center justify-between">
		<a class="text-sm text-primary truncate" {href}>
			{title}
		</a>

		<div class="flex flex-row gap-2 items-center">
			<slot name="actions" />
		</div>
	</div>
	<slot />
</div>
