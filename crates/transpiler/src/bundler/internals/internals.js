import { Reactive } from "signals";

export class Option {
	constructor(__, some) {
		this.__ = __;
		if (arguments.length > 1) this.some = some;
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
