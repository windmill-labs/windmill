<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import { concatCustomCss } from '../../utils'
	import type { AppInput, StaticInput } from '../../inputType'
	import type { AppEditorContext, ComponentCustomCSS } from '../../types'
	import { InputValue } from '../helpers'
	import { twMerge } from 'tailwind-merge'
	import Map from 'ol/Map'
	import View from 'ol/View'
	import TileLayer from 'ol/layer/Tile'
	import OSM from 'ol/source/OSM.js'
	import { fromLonLat, toLonLat } from 'ol/proj'

	export let id: string
	export let configuration: Record<string, AppInput>
	export const staticOutputs: string[] = ['loading']
	export let customCss: ComponentCustomCSS<'map'> | undefined = undefined

	const { app } = getContext<AppEditorContext>('AppEditorContext')
	let map: Map
	let mapElement: HTMLDivElement

	let longitude: number | undefined = undefined
	let latitude: number | undefined = undefined
	let zoom: number | undefined = undefined
	let markers: string | undefined = undefined

	$: if (map && longitude && latitude) {
		map.getView().setCenter(fromLonLat([longitude, latitude]))
	}

	$: if (map && zoom) {
		map.getView().setZoom(zoom)
	}

	onMount(() => {
		map = new Map({
			target: mapElement,
			layers: [
				new TileLayer({
					source: new OSM()
				})
			],
			view: new View({
				center: [longitude ?? 0, latitude ?? 0],
				zoom: zoom ?? 2
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
			const lonLat = toLonLat(center)
			;(configuration.longitude as StaticInput<number>).value = lonLat[0]
			;(configuration.latitude as StaticInput<number>).value = lonLat[1]
		})
	})

	$: css = concatCustomCss($app.css?.mapcomponent, customCss)
</script>

<InputValue {id} input={configuration.longitude} bind:value={longitude} />
<InputValue {id} input={configuration.latitude} bind:value={latitude} />
<InputValue {id} input={configuration.zoom} bind:value={zoom} />
<InputValue {id} input={configuration.markers} bind:value={markers} />

<div
	bind:this={mapElement}
	class={twMerge(`w-full h-full`, css?.map?.class ?? '')}
	style={css?.map?.style ?? ''}
/>
