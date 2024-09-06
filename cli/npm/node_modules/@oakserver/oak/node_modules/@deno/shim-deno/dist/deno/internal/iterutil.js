"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.merge = exports.filterAsync = exports.mapAsync = exports.map = void 0;
function* map(iter, f) {
    for (const i of iter) {
        yield f(i);
    }
}
exports.map = map;
async function* mapAsync(iter, f) {
    for await (const i of iter) {
        yield f(i);
    }
}
exports.mapAsync = mapAsync;
async function* filterAsync(iter, filter) {
    for await (const i of iter) {
        if (filter(i)) {
            yield i;
        }
    }
}
exports.filterAsync = filterAsync;
async function* merge(iterables) {
    const racers = new Map(map(map(iterables, (iter) => iter[Symbol.asyncIterator]()), (iter) => [iter, iter.next()]));
    while (racers.size > 0) {
        const winner = await Promise.race(map(racers.entries(), ([iter, prom]) => prom.then((result) => ({ result, iter }))));
        if (winner.result.done) {
            racers.delete(winner.iter);
        }
        else {
            yield await winner.result.value;
            racers.set(winner.iter, winner.iter.next());
        }
    }
}
exports.merge = merge;
