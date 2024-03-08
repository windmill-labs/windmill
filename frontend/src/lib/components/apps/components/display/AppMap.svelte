<script lang="ts">
	import { getContext } from 'svelte'
	import { initCss } from '../../utils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { InputValue } from '../helpers'
	import { twMerge } from 'tailwind-merge'
	import { Map, View, Feature } from 'ol'
	import { useGeographic } from 'ol/proj'
	import { OSM, Vector as VectorSource } from 'ol/source'
	import { Vector as VectorLayer, Tile as TileLayer } from 'ol/layer'
	import { Point } from 'ol/geom'
	import { defaults as defaultControls } from 'ol/control'
	import { findGridItem, initOutput } from '../../editor/appUtils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	interface Marker {
		lon: number
		lat: number
		title?: string
		radius?: number
		color?: string
		strokeWidth?: number
		strokeColor?: string
	}

	const LAYER_NAME = {
		MARKER: 'Marker'
	} as const

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'mapcomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore, selectedComponent, connectingInput, focusedGrid, mode } =
		getContext<AppViewerContext>('AppViewerContext')

	let outputs = initOutput($worldStore, id, {
		mapRegion: {
			topLeft: { lat: 0, lon: 0 },
			bottomRight: { lat: 0, lon: 0 }
		}
	})

	let map: Map | undefined = undefined
	let mapElement: HTMLDivElement | undefined = undefined

	let longitude: number | undefined = undefined
	let latitude: number | undefined = undefined
	let zoom: number | undefined = undefined
	// If string, it's a JSON file read as text
	let markers: Marker[] | string | undefined = undefined

	$: if (map && longitude && latitude) {
		map.getView().setCenter([longitude, latitude])
	}

	$: if (map && zoom) {
		map.getView().setZoom(zoom)
	}

	$: if (map && markers) {
		updateMarkers()
	}

	function selectComponent() {
		if (!$connectingInput.opened) {
			$selectedComponent = [id]
			$focusedGrid = undefined
		}
	}

	function getLayersByName(name: keyof typeof LAYER_NAME) {
		return map
			?.getLayers()
			?.getArray()
			?.filter((l) => l.getProperties().name === LAYER_NAME[name])
	}

	function getMarkerArray(): Marker[] | undefined {
		let array: Marker[] | undefined = undefined
		try {
			if (typeof markers === 'string') {
				const json = JSON.parse(markers)
				array = Array.isArray(json) ? json : [json]
			} else {
				array = markers
			}
			return array?.filter((m) => !isNaN(+m.lat) && !isNaN(+m.lon))
		} catch (error) {
			console.log(error)
			return undefined
		}
	}

	function createMarkerLayers() {
		const markerArray = getMarkerArray()
		return markerArray?.map((m) => {
			return new VectorLayer({
				properties: {
					name: LAYER_NAME.MARKER
				},
				source: new VectorSource({
					features: [
						new Feature({
							geometry: new Point([+m.lon, +m.lat]),
							name: m.title
						})
					]
				}),
				style: {
					'circle-radius': m.radius ?? 7,
					'circle-fill-color': m.color ?? '#dc2626',
					'circle-stroke-width': m.strokeWidth ?? 3,
					'circle-stroke-color': m.strokeColor ?? '#fca5a5'
				}
			})
		})
	}

	function updateMarkers() {
		const layers = getLayersByName('MARKER')
		if (layers?.length) {
			layers.forEach((l) => map?.removeLayer(l))
		}
		createMarkerLayers()?.forEach((l) => map?.addLayer(l))
	}

	$: if (!render) {
		map = undefined
		mapElement = undefined
	}

	$: if (!map && mapElement) {
		useGeographic()
		map = new Map({
			target: mapElement,
			layers: [
				new TileLayer({
					source: new OSM()
				}),
				...(createMarkerLayers() || [])
			],
			view: new View({
				center: [longitude ?? 0, latitude ?? 0],
				zoom: zoom ?? 2
			}),
			controls: defaultControls({
				attribution: false
			})
		})
		updateRegionOutput()
	}

	let css = initCss($app.css?.mapcomponent, customCss)

	function updateRegionOutput() {
		if (map) {
			let extent = map.getView().calculateExtent(map.getSize())
			const [left, bottom, right, top] = extent

			if (outputs?.mapRegion) {
				outputs.mapRegion.set({
					topLeft: { lat: top, lon: left },
					bottomRight: { lat: bottom, lon: right }
				})
			}
		}
	}

	function handleSyncRegion() {
		const gridItem = findGridItem($app, id)
		if (!map || !gridItem) {
			return
		}

		const z = map.getView().getZoom()
		updateRegionOutput()

		if (z) {
			//@ts-ignore
			gridItem.data.configuration.zoom.value = z
		}

		const center = map.getView().getCenter()
		if (!center) {
			return
		}

		if (gridItem) {
			//@ts-ignore
			gridItem.data.configuration.longitude.value = center[0]
			//@ts-ignore
			gridItem.data.configuration.latitude.value = center[1]
		}
	}
</script>

<InputValue key="longitude" {id} input={configuration.longitude} bind:value={longitude} />
<InputValue key="latitude" {id} input={configuration.latitude} bind:value={latitude} />
<InputValue key="zoom" {id} input={configuration.zoom} bind:value={zoom} />
<InputValue key="markers" {id} input={configuration.markers} bind:value={markers} />

<InitializeComponent {id} />

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.mapcomponent}
	/>
{/each}

{#if render}
	<div class="relative h-full w-full">
		<div
			on:pointermove={updateRegionOutput}
			on:wheel={updateRegionOutput}
			on:touchmove={updateRegionOutput}
			on:pointerdown|stopPropagation={selectComponent}
			bind:this={mapElement}
			class={twMerge(`w-full h-full`, css?.map?.class, 'wm-map')}
			style={css?.map?.style ?? ''}
		/>

		{#if $mode !== 'preview'}
			<div
				class="absolute bottom-0 left-0 px-1 py-0.5 bg-indigo-500 text-white text-2xs"
				on:pointerdown={handleSyncRegion}
			>
				Set region
			</div>
		{/if}
	</div>
{/if}

<style global lang="postcss">
	.ol-overlaycontainer-stopevent {
		@apply flex flex-col justify-start items-end;
	}
	.ol-control button {
		@apply w-7 h-7 center-center bg-surface border  text-secondary 
		rounded mt-1 mr-1 shadow duration-200 hover:bg-surface-hover focus:bg-surface-hover;
	}
</style>
