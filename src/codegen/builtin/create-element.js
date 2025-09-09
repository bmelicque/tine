function __createElement(tag, attributes, children) {
	const element = document.createElement(tag);
	for (const [key, value] in Object.entries(attributes)) {
		element.setAttribute(key, value ?? "");
	}
	for (const child of children) {
		element.append(child);
	}
	return element;
}
