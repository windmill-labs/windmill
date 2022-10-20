import { derived, get } from 'svelte/store'

export default function writableDerived(origins, derive, reflect, initial = undefined) {
	var childDerivedSetter,
		originValues,
		blockNextDerive = false

	var reflectOldValues = 'withOld' in reflect
	var wrappedDerive = (got, set) => {
		childDerivedSetter = set
		if (reflectOldValues) {
			originValues = got
		}
		if (!blockNextDerive) {
			let returned = derive(got, set)
			if (derive.length < 2) {
				set(returned)
			} else {
				return returned
			}
		}
		blockNextDerive = false
	}
	var childDerived = derived(origins, wrappedDerive, initial)

	var singleOrigin = !Array.isArray(origins)
	var sendUpstream = (setWith) => {
		if (singleOrigin) {
			blockNextDerive = true
			origins.set(setWith)
		} else {
			setWith.forEach((value, i) => {
				blockNextDerive = true
				origins[i].set(value)
			})
		}
		blockNextDerive = false
	}
	if (reflectOldValues) {
		reflect = reflect.withOld
	}
	var reflectIsAsync = reflect.length >= (reflectOldValues ? 3 : 2)
	var cleanup = null

	function doReflect(reflecting) {
		if (cleanup) {
			// @ts-ignore
			cleanup()
			cleanup = null
		}

		if (reflectOldValues) {
			var returned = reflect(reflecting, originValues, sendUpstream)
		} else {
			var returned = reflect(reflecting, sendUpstream)
		}
		if (reflectIsAsync) {
			if (typeof returned == 'function') {
				cleanup = returned
			}
		} else {
			sendUpstream(returned)
		}
	}

	var tryingSet = false
	function update(fn) {
		var isUpdated, mutatedBySubscriptions, oldValue, newValue
		if (tryingSet) {
			newValue = fn(get(childDerived))
			childDerivedSetter(newValue)
			return
		}
		var unsubscribe = childDerived.subscribe((value) => {
			if (!tryingSet) {
				oldValue = value
			} else if (!isUpdated) {
				isUpdated = true
			} else {
				mutatedBySubscriptions = true
			}
		})
		newValue = fn(oldValue)
		tryingSet = true
		childDerivedSetter(newValue)
		unsubscribe()
		tryingSet = false
		if (mutatedBySubscriptions) {
			newValue = get(childDerived)
		}
		if (isUpdated) {
			doReflect(newValue)
		}
	}
	return {
		subscribe: childDerived.subscribe,
		set(value) {
			update(() => value)
		},
		update
	}
}
