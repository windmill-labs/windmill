<script lang="ts">
	import { Map, View, Feature } from 'ol'
	import { Fill, Stroke, Style, Text } from 'ol/style.js'
	import { useGeographic } from 'ol/proj'
	import { OSM, Vector as VectorSource } from 'ol/source'
	import { Vector as VectorLayer, Tile as TileLayer } from 'ol/layer'
	import { Point } from 'ol/geom'
	import { defaults as defaultControls } from 'ol/control'
	import CircleStyle from 'ol/style/Circle'

	interface Marker {
		lon: number
		lat: number
		title?: string
		radius?: number
		color?: string
		strokeWidth?: number
		strokeColor?: string
	}

	export let lon: number | undefined = undefined
	export let lat: number | undefined = undefined
	export let zoom: number | undefined = undefined
	export let markers: Marker[] | string | undefined = undefined

	const LAYER_NAME = {
		MARKER: 'Marker'
	} as const

	let map: Map | undefined = undefined
	let mapElement: HTMLDivElement | undefined = undefined

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
							label: m.title,
							name: m.title
						})
					]
				}),
				style: new Style({
					image: new CircleStyle({
						radius: m.radius ?? 5,
						fill: new Fill({
							color: m.color ?? '#dc2626'
						}),
						stroke: new Stroke({
							color: m.strokeColor ?? '#fca5a5',
							width: m.strokeWidth ?? 3
						})
					}),
					text: new Text({
						text: m.title,
						offsetY: -15,
						fill: new Fill({
							color: '#000'
						})
					})
				})
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
				center: [lon ?? 0, lat ?? 0],
				zoom: zoom ?? 2
			}),
			controls: defaultControls({
				attribution: false
			})
		})
		if (lat && lon) {
			map.getView().setCenter([lon, lat])
		}

		if (map && zoom) {
			map.getView().setZoom(zoom)
		}

		if (map && markers) {
			updateMarkers()
		}
	}
</script>

<div bind:this={mapElement} class="w-full h-[300px]"></div>

<style global lang="postcss">
	.ol-overlaycontainer-stopevent {
		@apply flex flex-col justify-start items-end;
	}
	.ol-control button {
		@apply w-7 h-7 center-center bg-surface border  text-secondary 
		rounded mt-1 mr-1 shadow duration-200 hover:bg-surface-hover focus:bg-surface-hover;
	}
</style>
