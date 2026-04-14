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

	$get() {
		const $ = new this;
		$.$tag = this.$tag;
		if (this.$tag) {
			$._0 = typeof this._0 === "object" ? this._0.$get() : this._0;
		}
		return $;
	}

	$set(other) {
		if (this.$tag === 0) {
			if (other.$tag === 1) {
				this._0 = other._0;
			}
		} else {
			if (other.$tag === 0) {
				delete this._0;
			} else {
				typeof this._0 === "object" ? this._0.$set(other._0) : this._0 = other._0;
			}
		}

		this.$tag = other.$tag;
	}

	$assign(other) {
		if (this.$tag === 0) {
			if (other.$tag === 1) {
				this._0 = other._0;
			}
		} else {
			if (other.$tag === 0) {
				delete this._0;
			} else {
				this._0.$assign(other._0);
			}
		}

		this.$tag = other.$tag;
	}
}

export class Result {
	constructor(__, _) {
		this.__ = __;
		this[0] = _;
	}
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
