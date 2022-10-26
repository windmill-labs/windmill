export interface Subscriber {
    next(v: any),
}

export interface Observable {
    subscribe(x: Subscriber)

}
export interface Output extends Observable {
    set(x: any, force?: boolean): void
}

export function newSettableOutput(): Output {

    var value = null
    const subscribers: Subscriber[] = []

    function subscribe(x: Subscriber) {
        if (!subscribers.includes(x)) {
            subscribers.push(x)
        }
    }

    function set(x: any, force: boolean = false) {
        if (value != x || force) {
            value = x
            subscribers.forEach(x => x.next(value))
        }
    }


    return {
        subscribe,
        set,
    }
}