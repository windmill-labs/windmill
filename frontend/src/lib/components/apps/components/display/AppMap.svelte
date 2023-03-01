<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import { concatCustomCss } from '../../utils'
	import type { AppInput, StaticInput } from '../../inputType'
	import type { AppEditorContext, ComponentCustomCSS } from '../../types'
	import { InputValue } from '../helpers'
	import { twMerge } from 'tailwind-merge'
	import { Map, View, Feature } from 'ol'
	import { useGeographic } from 'ol/proj'
	import { OSM, Vector as VectorSource } from 'ol/source'
	import { Vector as VectorLayer, Tile as TileLayer } from 'ol/layer'
	import { Point } from 'ol/geom'
	import { defaults as defaultControls } from 'ol/control'

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
	export let configuration: Record<string, AppInput>
	export const staticOutputs: string[] = ['loading']
	export let customCss: ComponentCustomCSS<'map'> | undefined = undefined

	const { app, stateId, selectedComponent, connectingInput, focusedGrid } =
		getContext<AppEditorContext>('AppEditorContext')
	let map: Map
	let mapElement: HTMLDivElement

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
			$selectedComponent = id
			$focusedGrid = undefined
		}
	}

	function getLayersByName(name: keyof typeof LAYER_NAME) {
		return map
			.getLayers()
			.getArray()
			.filter((l) => l.getProperties().name === LAYER_NAME[name])
	}

	function getMarkerArray(): Marker[] | undefined {
		let array: Marker[] | undefined = undefined
		if (typeof markers === 'string') {
			try {
				array = JSON.parse(markers)
			} catch (e) {
				return undefined
			}
		} else {
			array = markers
		}
		return array?.filter((m) => !isNaN(+m.lat) && !isNaN(+m.lon))
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
			layers.forEach((l) => map.removeLayer(l))
		}
		createMarkerLayers()?.forEach((l) => map.addLayer(l))
	}

	onMount(() => {
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
		map.on('moveend', () => {
			const z = map.getView().getZoom()
			if (z) {
				;(configuration.zoom as StaticInput<number>).value = z
			}
			const center = map.getView().getCenter()
			if (!center) {
				return
			}
			;(configuration.longitude as StaticInput<number>).value = center[0]
			;(configuration.latitude as StaticInput<number>).value = center[1]

			$stateId++
		})
	})

	$: css = concatCustomCss($app.css?.mapcomponent, customCss)
</script>

<InputValue {id} input={configuration.longitude} bind:value={longitude} />
<InputValue {id} input={configuration.latitude} bind:value={latitude} />
<InputValue {id} input={configuration.zoom} bind:value={zoom} />
<InputValue {id} input={configuration.markers} bind:value={markers} />

<div
	on:pointerdown|stopPropagation={selectComponent}
	bind:this={mapElement}
	class={twMerge(`w-full h-full`, css?.map?.class ?? '')}
	style={css?.map?.style ?? ''}
/>

<style global lang="postcss">
	.ol-overlaycontainer-stopevent {
		@apply flex flex-col justify-start items-end;
	}
	.ol-control button {
		@apply w-7 h-7 center-center bg-white border border-gray-300 text-gray-700 
		rounded mt-1 mr-1 shadow duration-200 hover:bg-gray-100 focus:bg-gray-100 
		hover:border-gray-500 focus:border-gray-500;
	}
</style>
