import type { DynamicInput, StaticInput } from "./types"

export interface Subscriber<T> {
    next(v: T),
}

export interface Observable<T> {
    subscribe(x: Subscriber<T>)
}
export interface Output<T> extends Observable<T> {
    set(x: T, force?: boolean): void
}

export interface Input<T> extends Subscriber<T> {
    peak(): T | undefined
}

export function buildWorld() {
    function registerComponent() { }
}

export function buildObservableWorld() {
    const observables: Record<string, Output<any>> = {}

    function connect<T>(inputSpec: DynamicInput, next: (x: T) => void): Input<T> {
        const input = cachedInput(next)
        let obs = observables[`${inputSpec.id}.${inputSpec.name}`]
        if (!obs) {
            throw Error("Observable at " + inputSpec.id + "." + inputSpec.name + " not found")
        }
        obs.subscribe(input)
        return input
    }


    function newOutput<T>(id: string, name: string): Output<T> {
        const output = settableOutput<T>()
        output[`${id}.${name}`] = output
        return output
    }

    return {
        connect,
        newOutput
    }
}
export function cachedInput<T>(next: (x: T) => void): Input<T> {
    let value: T | undefined = undefined
    function peak(): T | undefined {
        return value
    }
    return {
        peak,
        next
    }

}

export function settableOutput<T>(): Output<T> {

    let value: T | undefined = undefined
    const subscribers: Subscriber<T>[] = []

    function subscribe(x: Subscriber<T>) {
        if (!subscribers.includes(x)) {
            subscribers.push(x)
        }
    }

    function set(x: T, force: boolean = false) {
        if (value != x || force) {
            value = x
            subscribers.forEach(x => x.next(value!))
        }
    }

    return {
        subscribe,
        set,
    }
}