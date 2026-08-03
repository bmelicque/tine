import { Reactive } from "signals";

export class Option {
	static None() {
		const $ = new this;
		$.$tag = 0;
		return $;
	}

	static Some(_0) {
		const $ = new this;
		$.$tag = 1;
		$._0 = _0;
		return $;
	}

	/**
	 * 
	 * @template T
	 * @param {T | undefined} value 
	 * @returns {Option<T>}
	 */
	static $from(value) {
		return t == null ? Option.None() : Option.Some(t)
	}

	$clone() {
		const $ = new this;
		$.$tag = this.$tag;
		if (this.$tag) {
			$._0 = this._0.$clone?.() ?? this._0;
		}
		return $;
	}
}

export class Result {
	constructor(__, _) {
		this.__ = __;
		this[0] = _;
	}
}

/**
 * Get the cloned value of given item.
 * @template T
 * @param {T} x 
 * @returns {T}
 */
export function clone(x) {
	if (typeof x !== "object" || !x) return x;

	if (Array.isArray(x)) return cloneArray(x);

	return x.$clone?.() ?? x
}

/**
 * Clone the given array.
 * @template T
 * @param {T[]} a
 * @returns {T[]}
 */
export function cloneArray(a) {
	const len = a.length;
	const out = new Array(len);
	for (let i = 0; i < len; i++) {
		const v = a[i];
		out[i] = typeof v === "object" && v ? get(v) : v;
	}
	return out;
}

/**
 * Replace the value at given index by given value.
 * negative indexes are counted from the end.
 * Return `true` if the new value can be inserted (even if it stays the same).
 * return `false` if the index is out of bounds.
 * @template T
 * @param {T[]} array 
 * @param {number} index 
 * @param {T} value 
 * @returns {boolean}
 */
function setArrayAt(array, index, value) {
	if (index < 0) index += array.length;
	if (index < 0 || index >= array.length) return false;
	array[index] = value;
	return true;
}

/**
 * Create a DOM Element, possibly reactive depending on the params used at creation.
 *
 * Children (inner nodes) can be created from:
 * - primitive values (`string`, `number`, etc.), that will be converted to Text nodes
 * - regular DOM nodes or elements
 * - reactive values (Signals or Listeners), that will be converted to reactive nodes
 * - any kind of expression evaluates to one of the above
 */
export function createElement(tag, attributes, children) {
	const element = document.createElement(tag);
	for (const [key, value] of Object.entries(attributes)) {
		if (key.startsWith("on")) element.addEventListener(key.slice(2), value);
		// TODO: reactive attributes
		else element.setAttribute(key, value ?? "");
	}
	if (children) {
		for (const child of children) {
			if (typeof child === "number") element.append(String(child));
			else if (child instanceof Reactive) element.append(child.toDOMNode().node);
			else element.append(child);
		}
	}

	return element;
}

/**
 * Generic combiner, based on MurmurHash3's block mixing.
 * Has good entropy on lower bits.
 * @param {number} h 
 * @param {number} k 
 * @returns {number}
 */
export function hashCombine(h, k) {
	k = Math.imul(k, 0xcc9e2d51);
	k = (k << 15) | (k >>> 17);
	k = Math.imul(k, 0x1b873593);
	h ^= k;
	h = (h << 13) | (h >>> 19);
	h = (Math.imul(h, 5) + 0xe6546b64) | 0;
	return h;
}

export function hashFinalize(h) {
	h ^= h >>> 16;
	h = Math.imul(h, 0x85ebca6b);
	h ^= h >>> 13;
	h = Math.imul(h, 0xc2b2ae35);
	h ^= h >>> 16;
	return h;
}

export function hashBool(value) {
	return value ? 1 : 0;
}

export function hashInt(value) {
	return value | 0
}

// Handling floats: use TypedArrays to handle a f64 as two i32.
const floatBuffer = new Float64Array(1);
const intView = new Int32Array(floatBuffer.buffer);
export function hashFloat(n) {
	floatBuffer[0] = n;
	return hashCombine(intView[0], intView[1]);
}

// Strings : FNV-1a
export function hashString(s) {
	let h = 0x811c9dc5 | 0;
	for (let i = 0; i < s.length; i++) {
		h ^= s.charCodeAt(i);
		h = Math.imul(h, 0x01000193);
	}
	return h;
}

export function hashAny(v) {
	if (v === null || v === undefined) return 0x9747b28c;
	const t = typeof v;
	if (t === "number") return hashFloat(v);
	if (t === "string") return hashString(v);
	if (t === "boolean") return hashBool(v);
	if (Array.isArray(v)) {
		let h = arr.length;
		for (let i = 0; i < arr.length; i++) {
			h = hashCombine(h, hashAny(arr[i]));
		}
		return h;
	};
	return v.hash();
}

export function eqAny(a, b) {
	if (a === b) return true;
	let ta = typeof a, tb = typeof b;
	if (ta !== tb || ta !== "object") return false;
	if (a.constructor !== b.constructor) return false;
	if (Array.isArray(a)) {
		if (a.length !== b.length) return false;
		for (let i = 0; i < a.length; i++)
			if (!eqAny(a[i], b[i])) return false;
		return true;
	}
	if ("eq" in a) return a.eq(b);
	return false;
}

const EMPTY = 0;
const OCCUPIED = 1;
const DELETED = 2; // tombstone

export class Map {
	constructor(capacity = 16) {
		this._resize(capacity);
	}

	/**
	 * Resize the whole map (also clears it)
	 * @param {number} capacity should be a power of two
	 */
	_resize(capacity) {
		this.capacity = capacity; // should always be a power of 2
		this.mask = capacity - 1;
		this.state = new Uint8Array(capacity);
		this.keys = new Array(capacity);
		this.values = new Array(capacity);
		this.count = 0; // truly occupied slots
		this.tombstones = 0; // deleted slots
	}

	/**
	 * Probe the map to find if the key was already inserted
	 * @param {unknown} key
	 * @returns {number} positive if found, else negative = `-(insertPoint + 1)`
	 */
	_probe(key) {
		const h = hashAny(key) >>> 0;
		let idx = h & this.mask;
		let firstTombstone = -1;

		for (let i = 0; i < this.capacity; i++) {
			const s = this.state[idx];

			switch (s) {
				case EMPTY:
					// Not found, returning either this or the first found tombstone
					const i = firstTombstone !== -1 ? firstTombstone : idx;
					return -(i + 1);
				case DELETED:
					if (firstTombstone === -1) firstTombstone = idx;
					break;
				case OCCUPIED:
					if (eqAny(this.keys[idx], key)) return idx;
					break;
			}

			idx = (idx + 1) & this.mask; // linear probing; `& mask` = wraparound
		}

		throw new Error("hash table full");
	}

	get(key) {
		const r = this._probe(key);
		return r >= 0 ? this.values[r] : undefined;
	}

	insert(key, value) {
		this._maybeGrow();
		let r = this._probe(key);
		if (r < 0) {
			r = -(r + 1);
			if (this.state[r] === DELETED) this.tombstones--;
			this.state[r] = OCCUPIED;
			this.keys[r] = key;
			this.count++;
		}
		this.values[r.idx] = value;
	}

	delete(key) {
		const r = this._probe(key);
		if (r < 0) return false;
		this.state[r] = DELETED;
		// release the key and value
		this.keys[r] = undefined;
		this.values[r] = undefined;
		this.count--;
		this.tombstones++;
		return true;
	}

	// Grow if occupied+tombstones is more than ~70% of capacity.
	// Tombstones are counted, or else table with lots of deletes/sets would
	// never rehash and probe chains would grow indefinitely.
	_maybeGrow() {
		if ((this.count + this.tombstones + 1) / this.capacity <= 0.7) return;

		const oldKeys = this.keys,
			oldValues = this.values,
			oldState = this.state,
			oldCapacity = this.capacity;
		// double only if more occupied than tombstones (else a compaction will
		// be better)
		const shouldDouble = this.count / oldCapacity > 0.35;
		this._resize(shouldDouble ? oldCapacity * 2 : oldCapacity);

		for (let i = 0; i < oldCapacity; i++) {
			if (oldState[i] === OCCUPIED) {
				this._insertRehashed(oldKeys[i], oldValues[i]);
			}
		}
	}

	_insertRehashed(key, value) {
		const h = hashAny(key) >>> 0;
		let idx = h & this.mask;

		while (this.state[idx] === OCCUPIED) {
			idx = (idx + 1) & this.mask;
		}

		this.state[idx] = OCCUPIED;
		this.keys[idx] = key;
		this.values[idx] = value;
		this.count++;
	}
}
