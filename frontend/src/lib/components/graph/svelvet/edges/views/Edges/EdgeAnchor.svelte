<script lang="ts">
	export let x: number
	export let y: number
</script>

<!-- renders simple half-circle for the anchor point of the edge -->

<circle cx={x} cy={y} r={5} stroke="white" fill="black" />

<style>
	circle {
		position: absolute;
	}
</style>
