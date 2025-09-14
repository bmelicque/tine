class __Reference {
	constructor(ctx, value) {
		this.ctx = ctx;
		this.value = value;
	}

	get() {
		return this.ctx?.[this.value] ?? this.value;
	}

	set(value) {
		this.ctx ? (this.ctx[this.value] = value) : (this.value = value);
	}
}
