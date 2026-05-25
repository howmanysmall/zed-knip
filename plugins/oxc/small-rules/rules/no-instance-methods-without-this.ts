import { isNode } from "@oxlint-utilities/oxc-utilities";
import { isRecord } from "@oxlint-utilities/type-utilities";
import { defineRule } from "oxlint-plugin-utilities";

import type { ESTree, Visitor } from "oxlint-plugin-utilities";

interface Options {
	readonly checkPrivate?: boolean;
	readonly checkProtected?: boolean;
	readonly checkPublic?: boolean;
}

type NormalizedOptions = Readonly<Required<Options>>;

const DEFAULT_OPTIONS: NormalizedOptions = {
	checkPrivate: true,
	checkProtected: true,
	checkPublic: true,
};

function normalizeOptions(rawOptions: unknown): NormalizedOptions {
	if (!isRecord(rawOptions)) return DEFAULT_OPTIONS;

	return {
		checkPrivate: typeof rawOptions.checkPrivate === "boolean" ? rawOptions.checkPrivate : true,
		checkProtected: typeof rawOptions.checkProtected === "boolean" ? rawOptions.checkProtected : true,
		checkPublic: typeof rawOptions.checkPublic === "boolean" ? rawOptions.checkPublic : true,
	};
}

function shouldCheckMethod(node: ESTree.MethodDefinition, options: NormalizedOptions): boolean {
	if (node.static || node.kind !== "method") return false;

	const accessibility = node.accessibility ?? "public";
	if (accessibility === "private" && !options.checkPrivate) return false;
	if (accessibility === "protected" && !options.checkProtected) return false;
	if (accessibility === "public" && !options.checkPublic) return false;

	return true;
}

function traverseForThis(currentNode: ESTree.Node, visited: WeakSet<ESTree.Node>): boolean {
	if (visited.has(currentNode)) return false;

	visited.add(currentNode);
	if (currentNode.type === "ThisExpression" || currentNode.type === "Super") return true;
	if (!isRecord(currentNode)) return false;

	for (const key in currentNode) {
		if (!Object.hasOwn(currentNode, key)) continue;

		const childValue = currentNode[key];
		if (childValue === undefined) continue;

		if (Array.isArray(childValue)) {
			for (const item of childValue) if (isNode(item) && traverseForThis(item, visited)) return true;
			continue;
		}

		if (isNode(childValue) && traverseForThis(childValue, visited)) return true;
	}

	return false;
}

function methodUsesThis({ value }: ESTree.MethodDefinition): boolean {
	if (value.type !== "FunctionExpression") return false;
	return traverseForThis(value, new WeakSet());
}

function getMethodName(node: ESTree.MethodDefinition): string {
	return node.key.type === "Identifier" ? node.key.name : "unknown";
}

const noInstanceMethodsWithoutThis = defineRule({
	create(context): Visitor {
		const options = normalizeOptions(context.options[0]);

		return {
			MethodDefinition(node: ESTree.MethodDefinition): void {
				if (!shouldCheckMethod(node, options) || methodUsesThis(node)) return;

				context.report({
					data: { methodName: getMethodName(node) },
					messageId: "noInstanceMethodWithoutThis",
					node,
				});
			},
		} satisfies Visitor;
	},
	meta: {
		docs: {
			description:
				"Detect instance methods that do not use 'this' and suggest converting them to standalone functions for better performance in roblox-ts.",
		},
		messages: {
			noInstanceMethodWithoutThis:
				"Method '{{methodName}}' does not use 'this' and creates unnecessary metatable overhead in roblox-ts. Convert it to a standalone function for better performance.",
		},
		schema: [
			{
				additionalProperties: false,
				properties: {
					checkPrivate: {
						default: true,
						description: "Check private methods (default: true)",
						type: "boolean",
					},
					checkProtected: {
						default: true,
						description: "Check protected methods (default: true)",
						type: "boolean",
					},
					checkPublic: {
						default: true,
						description: "Check public methods (default: true)",
						type: "boolean",
					},
				},
				type: "object",
			},
		],
		type: "problem",
	},
});

export default noInstanceMethodsWithoutThis;
