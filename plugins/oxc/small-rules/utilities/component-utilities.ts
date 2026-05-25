import { isUppercaseName } from "@oxlint-utilities/string-utilities";

import type { ESTree } from "oxlint-plugin-utilities";

export function isComponentDeclaration(node: ESTree.Node): boolean {
	return node.type === "FunctionDeclaration" && node.id !== null && isUppercaseName(node.id.name);
}

export function isMemoCall(node: ESTree.Node): boolean {
	if (node.type !== "CallExpression") return false;
	if (node.callee.type === "Identifier") return node.callee.name === "memo";

	return (
		node.callee.type === "MemberExpression" &&
		node.callee.object.type === "Identifier" &&
		node.callee.object.name === "React" &&
		node.callee.property.type === "Identifier" &&
		node.callee.property.name === "memo"
	);
}

export function isSimpleExpression(node: ESTree.Node): boolean {
	if (node.type === "BinaryExpression") {
		switch (node.operator) {
			case "%":
			case "*":
			case "**":
			case "+":
			case "-":
			case "/":
				return isSimpleExpression(node.left) && isSimpleExpression(node.right);
			default:
				return false;
		}
	}

	if (node.type === "Identifier" || node.type === "Literal") return true;
	if (node.type === "MemberExpression") return !node.computed && isSimpleExpression(node.object);
	if (node.type === "ParenthesizedExpression") return isSimpleExpression(node.expression);
	if (node.type === "TemplateLiteral") return node.expressions.length === 0;
	if (node.type === "UnaryExpression") return isSimpleExpression(node.argument);
	return false;
}

export function isHoistableJSXElementName(
	name: ESTree.JSXElementName,
	additionalComponents: ReadonlySet<string>,
): boolean {
	if (name.type !== "JSXIdentifier") return false;
	const firstCharacter = name.name.charAt(0);
	if (firstCharacter !== "" && firstCharacter === firstCharacter.toLowerCase()) return true;
	return additionalComponents.has(name.name);
}
