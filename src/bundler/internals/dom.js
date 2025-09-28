/**
 * Insert an Element within the first node that matches the given CSS selector
 * @param {string} selector a CSS selector designating where should the element be appended
 * @param {Element} element
 */
export function render(selector, element) {
	const root = document.querySelector(selector);
	if (!root) return new Result("Error", "Invalid root selector");
	root.append(element);
}
