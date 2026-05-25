import { unwrapExpression } from "@oxlint-utilities/ast-utilities";
import { isKeyOfNode, isNode } from "@oxlint-utilities/oxc-utilities";

import type { CallbackFunction } from "@oxlint-types/missing-types";
import type { ESTree, SourceCode } from "oxlint-plugin-utilities";

const SETTER_IDENTIFIER_PATTERN = /^set[A-Z]/u;

export function getHookName({ callee }: ESTree.CallExpression): string | undefined {
	if (callee.type === "Identifier") return callee.name;
	if (callee.type === "MemberExpression" && callee.property.type === "Identifier") return callee.property.name;
	return undefined;
}

export function isSetterIdentifier(name: string): boolean {
	return SETTER_IDENTIFIER_PATTERN.test(name);
}

export function getEffectCallback(callExpression: ESTree.CallExpression): CallbackFunction | undefined {
	const [callback] = callExpression.arguments;
	return callback?.type === "ArrowFunctionExpression" || callback?.type === "FunctionExpression"
		? callback
		: undefined;
}

export function walkAst(node: ESTree.Node, callback: (child: ESTree.Node) => void): void {
	const stack = [node];
	while (stack.length > 0) {
		const current = stack.pop();
		if (current === undefined) break;
		callback(current);

		for (const key in current) {
			if (isKeyOfNode(key)) continue;

			const value: unknown = Reflect.get(current, key);
			if (typeof value !== "object" || value === null) continue;

			if (value === current.parent) continue;

			if (Array.isArray(value)) {
				for (let index = value.length - 1; index >= 0; index -= 1) {
					const item = value[index];
					// oxlint-disable-next-line max-depth
					if (isNode(item)) stack.push(item);
				}
				continue;
			}

			if (isNode(value)) stack.push(value);
		}
	}
}

export function walkAstSlop(node: ESTree.Node, callback: (child: ESTree.Node) => void): void {
	callback(node);

	for (const child of Object.values(node)) {
		if (Array.isArray(child)) {
			for (const item of child) {
				if (item === node.parent) continue;
				if (isNode(item)) walkAstSlop(item, callback);
			}

			continue;
		}

		if (child === node.parent) continue;
		if (isNode(child)) walkAstSlop(child, callback);
	}
}

export function countSetStateCalls(node: ESTree.Node): number {
	let count = 0;

	walkAst(node, (child) => {
		if (child.type !== "CallExpression" || child.callee.type !== "Identifier") return;
		if (isSetterIdentifier(child.callee.name)) count += 1;
	});

	return count;
}

export const enum DependenciesKind {
	MissingOrOmitted = 0,
	EmptyArray = 1,
	StaticArray = 2,
	DynamicOrUnknown = 3,
}
export type IsStaticArrayExpression<TOptions extends object> = (
	sourceCode: SourceCode,
	arrayExpression: ESTree.ArrayExpression,
	seen: Set<ESTree.Node>,
	options: TOptions,
) => boolean;
export function classifyDependencies<TOptions extends object>(
	sourceCode: SourceCode,
	argument: ESTree.Argument | undefined,
	seen: Set<ESTree.Node>,
	options: TOptions,
	isStaticArrayExpression: IsStaticArrayExpression<TOptions>,
): DependenciesKind {
	if (argument === undefined) return DependenciesKind.MissingOrOmitted;
	if (argument.type === "SpreadElement") return DependenciesKind.DynamicOrUnknown;

	const expression = unwrapExpression(argument);
	if (expression.type !== "ArrayExpression") return DependenciesKind.DynamicOrUnknown;
	if (expression.elements.length === 0) return DependenciesKind.EmptyArray;
	if (isStaticArrayExpression(sourceCode, expression, seen, options)) return DependenciesKind.StaticArray;

	return DependenciesKind.DynamicOrUnknown;
}
