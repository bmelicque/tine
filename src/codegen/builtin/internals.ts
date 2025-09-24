/**
 * GENERAL STRATEGY FOR REACTIVE VALUES
 *
 * When a Signal is set:
 * 1. Mark all its descendants as being dirty
 * 2. Schedule a global update
 *
 * When a global update is run:
 * 1. Build the list of nodes to recompute (the 'stale' list ) by taking all dirty nodes of depth==1
 * 2. Recompute the value of each dirty node in the list, and if the value has changed add its children to the list
 * 3. Drop the dirty and stale lists. Dirty yet not recomputed nodeare in branches that have been cut-off early (see next point)
 *
 * If a Computed is read while marked as dirty, but global update has not started:
 * 1. Recompute the value (this will also recompute any ancestor)
 * 2. Remove any recomputed value form the 'dirty' list
 * This might leave some descendants dirty but not accessible through the global update: their value wasn't supposed to change anyways.
 */

//
type Getter<T> = () => T;
type Setter<T> = (value: T) => void;

/**
 * Stores reactive graph nodes, organized in layers.
 *
 * Each layer corresponds to a depth within the graph.
 * This way, nodes can be inserted in any order, but still be iterated layer by layer (breadth first)
 */
class LayeredGraph<T extends Reactive> {
	layers: Set<T>[] = [];

	/**
	 * Check if a node exists within the graph
	 */
	has(node: T): boolean {
		return this.layers[node.depth]?.has(node);
	}

	/**
	 * Add a node to the graph.
	 *
	 * The node is automatically inserted in the layer corresponding the node's `depth`
	 */
	add(node: T) {
		(this.layers[node.depth] ??= new Set()).add(node);
	}

	/**
	 * Remove a node from the `LayeredGraph`
	 */
	remove(node: T) {
		this.layers[node.depth]?.delete(node);
	}

	/**
	 * Empties the `LayeredGraph`
	 */
	clear() {
		this.layers.length = 0;
	}
}

const stale = new LayeredGraph<Listener<any>>();
/**
 * Graph of all the reactive values for which at least one ancestor's value has changed since last global update.
 */
const dirty = new LayeredGraph<Listener<any>>();

const scheduler = {
	hasScheduled: false,

	schedule() {
		if (this.hasScheduled) return;
		this.hasScheduled = true;
		queueMicrotask(() => this.updateTree());
	},

	updateTree() {
		this.hasScheduled = false;
		if (!dirty.layers[1]) return;
		stale.layers[1] = dirty.layers[1];
		for (const depth of stale.layers) {
			for (const node of depth.values()) {
				node.update();
			}
		}
		stale.clear();
		dirty.clear();
	},
};

/**
 * The basic reactive class, which provides graph utilities.
 */
abstract class Reactive {
	depth: number;
	children = new Set<WeakRef<Listener<any>>>();
	registry = new FinalizationRegistry<WeakRef<Listener<any>>>((ref) => this.children.delete(ref));

	addChild(computed: Listener<any>) {
		const ref = new WeakRef(computed);
		this.children.add(ref);
		this.registry.register(computed, ref);
	}

	*iterateChildren() {
		for (const ref of this.children) {
			const child = ref.deref();
			if (child) yield child;
		}
	}
}

/**
 * Root reactive states, that are writable and do not depend on anything
 */
export class Signal<T> extends Reactive {
	depth = 0;
	value: T;

	constructor(value: T) {
		super();
		this.value = value;
	}

	get() {
		return this.value;
	}

	set(value: T) {
		this.value = value;
		this.setupTreeUpdate();
	}

	setupTreeUpdate() {
		for (const child of this.iterateChildren()) child.dirty();
		scheduler.schedule();
	}
}

/**
 * Derived signals and effects
 */
export class Listener<T> extends Reactive {
	getter: Getter<T>;
	deps: Reactive[];
	value: T;

	constructor(deps: Reactive[], getter: Getter<T>) {
		super();
		this.getter = getter;
		this.deps = deps;
		let depth = 0;
		for (const dep of deps) {
			if (dep.depth >= depth) depth = dep.depth + 1;
			dep.addChild(this);
		}
		this.depth = depth;
		this.value = getter();
	}

	/**
	 * Update cached value
	 * @returns `true` if cached value has changed
	 */
	compute() {
		const old = this.value;
		this.value = this.getter();
		dirty.remove(this);
		return this.value !== old;
	}

	get() {
		if (dirty.has(this)) this.compute();
		return this.value;
	}

	/**
	 * Mark all the subtree as dirty
	 */
	dirty() {
		dirty.add(this);
		for (const child of this.iterateChildren()) child.dirty();
	}

	update() {
		if (!dirty.has(this) || !this.compute()) return;
		for (const child of this.iterateChildren()) stale.add(child);
	}
}

/**
 * Writable derived states, which should only be sub-values (like obj.value, or tuple[index]).
 * They depend on a single state and can write into it.
 */
export class WritableComputed<T> extends Listener<T> {
	/**
	 * Example setter for `&obj.value`:
	 * ```
	 * (newValue) => { obj.get().value = newValue }
	 * ```
	 */
	setter: Setter<T>;
	declare deps: [Signal<any>];

	constructor(deps: Reactive[], getter: Getter<T>, setter: Setter<T>) {
		super(deps, getter);
		this.setter = setter;
	}

	set(value: T) {
		this.setter(value);
		this.value = value;
		const rootState = this.deps[0];
		rootState.setupTreeUpdate();
	}
}

/**
 * Reactive DOM node
 */
export class ReactiveNode<T> extends Listener<T> {
	node: Node;

	constructor(deps: Reactive[], getter: Getter<T>) {
		super(deps, getter);
		this.node = this.toNode();
	}

	toNode() {
		return this.value instanceof Node ? this.value : new Text(String(this.value || ""));
	}

	update() {
		if (!dirty.has(this) || !this.compute()) return;
		for (const child of this.iterateChildren()) stale.add(child);
		this.node.parentNode!.replaceChild(this.node, (this.node = this.toNode()));
	}
}

/**
 *  DOM ELEMENT CREATION
 */
// TODO:
type Attributes = Record<string, any>;
type Child = string | number | Node | ReactiveNode<any>;

/**
 * Create a DOM Element, possibly reactive depending on the params used at creation.
 *
 * Children (inner nodes) can be created from:
 * - primitive values (`string`, `number`, etc.), that will be converted to Text nodes
 * - regular DOM nodes or elements
 * - reactive values (Signals or Listeners), that will be converted to reactive nodes
 * - any kind of expression evaluates to one of the above
 */
export function createElement(tag: string, attributes: Attributes, children: Child[]): Element {
	const element = document.createElement(tag);
	for (const [key, value] of Object.entries(attributes)) {
		// TODO: reactive attributes
		element.setAttribute(key, value ?? "");
	}
	if (children) {
		for (const child of children) {
			if (typeof child === "number") element.append(String(child));
			else if (child instanceof ReactiveNode) element.append(child.node);
			else element.append(child);
		}
	}

	return element;
}
