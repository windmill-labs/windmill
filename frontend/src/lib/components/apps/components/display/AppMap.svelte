<script lang="ts">
	import { getContext } from 'svelte'
	import { initCss } from '../../utils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { twMerge } from 'tailwind-merge'
	import { Map, View, Feature } from 'ol'
	import { useGeographic } from 'ol/proj'
	import { OSM, Vector as VectorSource } from 'ol/source'
	import { Vector as VectorLayer, Tile as TileLayer } from 'ol/layer'
	import { Point } from 'ol/geom'
	import { defaults as defaultControls } from 'ol/control'
	import { findGridItem, initConfig, initOutput } from '../../editor/appUtils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { Style, Circle, Fill, Stroke, Text } from 'ol/style'
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
	export let extraKey: string | undefined = undefined

	const { app, worldStore, selectedComponent, connectingInput, focusedGrid, mode } =
		getContext<AppViewerContext>('AppViewerContext')

	const resolvedConfig = initConfig(
		components['mapcomponent'].initialData.configuration,
		configuration
	)

	let outputs = initOutput($worldStore, id, {
		mapRegion: {
			topLeft: { lat: 0, lon: 0 },
			bottomRight: { lat: 0, lon: 0 }
		}
	})

	let map: Map | undefined = undefined
	let mapElement: HTMLDivElement | undefined = undefined

	$: if (map && resolvedConfig.longitude && resolvedConfig.latitude) {
		map.getView().setCenter([resolvedConfig.longitude, resolvedConfig.latitude])
	}

	$: if (map && resolvedConfig.zoom) {
		map.getView().setZoom(resolvedConfig.zoom)
	}

	$: if (map && resolvedConfig.markers) {
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
			if (typeof resolvedConfig.markers === 'string') {
				const json = JSON.parse(resolvedConfig.markers)
				array = Array.isArray(json) ? json : [json]
			} else {
				array = resolvedConfig.markers
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
			const feature = new Feature({
				geometry: new Point([+m.lon, +m.lat]),
				name: m.title
			})

			feature.setStyle(
				new Style({
					image: new Circle({
						radius: m.radius ?? 7,
						fill: new Fill({ color: m.color ?? '#dc2626' }),
						stroke: new Stroke({ width: m.strokeWidth ?? 3, color: m.strokeColor ?? '#fca5a5' })
					}),
					text: new Text({
						text: m.title,
						font: '12px Calibri,sans-serif',
						fill: new Fill({ color: '#000' }),
						stroke: new Stroke({
							color: '#fff',
							width: 2
						}),
						offsetY: -15
					})
				})
			)

			return new VectorLayer({
				properties: {
					name: LAYER_NAME.MARKER
				},
				source: new VectorSource({
					features: [feature]
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
				center: [resolvedConfig.longitude ?? 0, resolvedConfig.latitude ?? 0],
				zoom: resolvedConfig.zoom ?? 2
			}),
			controls: defaultControls({
				attribution: false
			})
		})
		updateRegionOutput()
	}

	let previousLock: boolean | undefined = undefined

	function updateInteractions(map: Map) {
		if (previousLock === resolvedConfig.lock) {
			return
		}

		previousLock = resolvedConfig.lock

		map.getInteractions().forEach((i) => {
			i.setActive(!resolvedConfig.lock)
		})

		map.changed()
	}

	$: map && resolvedConfig && updateInteractions(map)

	let css = initCss($app.css?.mapcomponent, customCss)

	function updateRegionOutput() {
		if (map && !resolvedConfig.lock) {
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

	async function handleSyncRegion() {
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
			gridItem.data.configuration.latitude.value = center[1] // $app = $app
		}
	}
</script>

{#each Object.entries(components['mapcomponent'].initialData.configuration) as [key, initialConfig] (key)}
	<ResolveConfig
		{id}
		{extraKey}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
		{initialConfig}
	/>
{/each}

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
	<div class="relative h-full w-full component-wrapper">
		<div
			on:pointermove={updateRegionOutput}
			on:wheel={updateRegionOutput}
			on:touchmove={updateRegionOutput}
			on:pointerdown|stopPropagation={selectComponent}
			bind:this={mapElement}
			class={twMerge(`w-full h-full`, css?.map?.class, 'wm-map')}
			style={css?.map?.style ?? ''}
		></div>

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
