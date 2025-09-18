/**
 * GENERAL STRATEGY
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
interface Settable<T> {
	set(value: T): void;
}
type Getter<T> = () => T;
type Setter<T> = (value: T) => void;

/**
 * Stores reactive graph nodes, sorted by depth
 */
class LayeredGraph<T extends Reactive> {
	layers: Set<T>[] = [];

	has(node: T) {
		return this.layers[node.depth]?.has(node);
	}

	add(node: T) {
		(this.layers[node.depth] ??= new Set()).add(node);
	}

	remove(node: T) {
		this.layers[node.depth]?.delete(node);
	}

	clear() {
		this.layers.length = 0;
	}
}

const stale = new LayeredGraph<__Computed<any>>();
const dirty = new LayeredGraph<__Computed<any>>();

const scheduler = {
	hasScheduled: false,

	schedule() {
		if (this.hasScheduled) return;
		this.hasOne = true;
		queueMicrotask(() => this.updateTree());
	},

	updateTree() {
		this.hasOne = false;
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
 * The basic reactive class, which provides graph utilities
 */
abstract class Reactive {
	depth: number;
	children = new Set<WeakRef<__Computed<any>>>();
	registry = new FinalizationRegistry<WeakRef<__Computed<any>>>((ref) => this.children.delete(ref));

	addChild(computed: __Computed<any>) {
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
export class __State<T> extends Reactive {
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
export class __Computed<T> extends Reactive {
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
export class __WritableComputed<T> extends __Computed<T> {
	/**
	 * Example setter for `&obj.value`:
	 * ```
	 * (newValue) => { obj.get().value = newValue }
	 * ```
	 */
	setter: Setter<T>;
	declare deps: [__State<any>];

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
export class __ReactiveNode<T> extends __Computed<T> {
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
