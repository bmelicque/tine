import { Reactive } from "signals";

export class Option {
	None() {
		const $ = new this;
		$.$tag = 0;
		return $;
	}

	Some(_0) {
		const $ = new this;
		$.$tag = 1;
		$._0 = _0;
		return $;
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
