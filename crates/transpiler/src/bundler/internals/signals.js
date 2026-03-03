
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

/**
 * Stores reactive graph nodes, organized in layers.
 *
 * Each layer corresponds to a depth within the graph.
 * This way, nodes can be inserted in any order, but still be iterated layer by layer (breadth first)
 */
class LayeredGraph {
	layers = [];

	/**
	 * Check if a node exists within the graph
	 */
	has(node) {
		return this.layers[node.depth]?.has(node);
	}

	/**
	 * Add a node to the graph.
	 *
	 * The node is automatically inserted in the layer corresponding the node's `depth`
	 */
	add(node) {
		(this.layers[node.depth] ??= new Set()).add(node);
	}

	/**
	 * Remove a node from the `LayeredGraph`
	 */
	remove(node) {
		this.layers[node.depth]?.delete(node);
	}

	/**
	 * Empties the `LayeredGraph`
	 */
	clear() {
		this.layers.length = 0;
	}
}

const stale = new LayeredGraph();
/**
 * Graph of all the reactive values for which at least one ancestor's value has changed since last global update.
 */
const dirty = new LayeredGraph();

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
			if (!depth) continue;
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
export class Reactive {
	children = new Set();
	registry = new FinalizationRegistry((ref) => this.children.delete(ref));

	addChild(computed) {
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

    toDOMNode() {
        return this instanceof ReactiveNode ? this : new ReactiveNode([this], () => this.get());
    }
}

/**
 * Root reactive states, that are writable and do not depend on anything
 */
export class Signal extends Reactive {
	depth = 0;

	constructor(value) {
		super();
		this.value = value;
	}

	get() {
		return this.value;
	}

	set(value) {
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
export class Listener extends Reactive {
	constructor(deps, getter) {
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
export class WritableComputed extends Listener {
	/**
	 * Example setter for `&obj.value`:
	 * ```
	 * (newValue) => { obj.get().value = newValue }
	 * ```
	 */
	constructor(deps, getter, setter) {
		super(deps, getter);
		this.setter = setter;
	}

	set(value) {
		this.setter(value);
		this.value = value;
		const rootState = this.deps[0];
		rootState.setupTreeUpdate();
	}
}

/**
 * Reactive DOM node
 */
export class ReactiveNode extends Listener {
	static signalKey = Symbol();

	constructor(deps, getter) {
		super(deps, getter);
		this.node = this.toNode();
	}

	toNode() {
		const node = this.value instanceof Node ? this.value : new Text(String(this.value ?? ""));
		// This prevents the ReactiveNode from being garbage collected
		// while the associated node is still in the DOM
		node[ReactiveNode.signalKey] = this;
		return node;
	}

	update() {
		if (!dirty.has(this) || !this.compute()) return;
		for (const child of this.iterateChildren()) stale.add(child);
		const newNode = this.toNode();
		this.node.parentNode.replaceChild(newNode, this.node);
		this.node = newNode;
	}
}

export function state(initialValue) {
    return new Signal(initialValue);
}

export function derived$(getter, dependencies) {
    return new Listener(dependencies, getter);
}