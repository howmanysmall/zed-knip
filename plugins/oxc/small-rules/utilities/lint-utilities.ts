import { isUppercaseName } from "@oxlint-utilities/string-utilities";

import type { ESTree } from "oxlint-plugin-utilities";

export function isHookCall(node: ESTree.Node | null, hookName: ReadonlySet<string> | string): boolean {
	return (
		node?.type === "CallExpression" &&
		node.callee.type === "Identifier" &&
		(typeof hookName === "string" ? node.callee.name === hookName : hookName.has(node.callee.name))
	);
}

export function isComponentAssignment(node: ESTree.Node): boolean {
	return (
		node.type === "VariableDeclarator" &&
		node.id.type === "Identifier" &&
		isUppercaseName(node.id.name) &&
		node.init !== null &&
		(node.init.type === "ArrowFunctionExpression" || node.init.type === "FunctionExpression")
	);
}
