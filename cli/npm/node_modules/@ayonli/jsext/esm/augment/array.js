import { first, last, random, count, equals, includesSlice, startsWith, endsWith, split, chunk, unique, uniqueBy, shuffle, orderBy, groupBy, keyBy, partition } from '../array.js';

Array.prototype.first = function first$1() {
    return first(this);
};
Array.prototype.last = function last$1() {
    return last(this);
};
Array.prototype.random = function random$1(remove = false) {
    return random(this, remove);
};
Array.prototype.count = function count$1(ele) {
    return count(this, ele);
};
Array.prototype.equals = function equals$1(another) {
    return equals(this, another);
};
Array.prototype.includesSlice = function includesSlice$1(slice) {
    return includesSlice(this, slice);
};
Array.prototype.startsWith = function startsWith$1(prefix) {
    return startsWith(this, prefix);
};
Array.prototype.endsWith = function endsWith$1(suffix) {
    return endsWith(this, suffix);
};
Array.prototype.split = function split$1(delimiter) {
    return split(this, delimiter);
};
Array.prototype.chunk = function chunk$1(length) {
    return chunk(this, length);
};
Array.prototype.unique = Array.prototype.uniq = function unique$1() {
    return unique(this);
};
Array.prototype.uniqueBy = Array.prototype.uniqBy = function uniqueBy$1(fn) {
    return uniqueBy(this, fn);
};
Array.prototype.shuffle = function shuffle$1() {
    return shuffle(this);
};
Array.prototype.toShuffled = function toShuffled() {
    return this.slice().shuffle();
};
if (!Array.prototype.toReversed) {
    Array.prototype.toReversed = function toReversed() {
        return this.slice().reverse();
    };
}
if (!Array.prototype.toSorted) {
    Array.prototype.toSorted = function toSorted(fn) {
        return this.slice().sort(fn);
    };
}
Array.prototype.orderBy = function orderBy$1(key, order = "asc") {
    return orderBy(this, key, order);
};
Array.prototype.groupBy = function groupBy$1(fn, type = Object) {
    return groupBy(this, fn, type);
};
Array.prototype.keyBy = function keyBy$1(fn, type = Object) {
    return keyBy(this, fn, type);
};
Array.prototype.partition = function partition$1(predicate) {
    return partition(this, predicate);
};
//# sourceMappingURL=array.js.map
