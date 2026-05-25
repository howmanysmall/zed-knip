// oxlint-disable typescript/no-unnecessary-condition
import { defineRule } from "oxlint-plugin-utilities";

import type { ESTree, SourceCode, Visitor } from "oxlint-plugin-utilities";

type SizeCallExpression = ESTree.CallExpression & {
	readonly callee: ESTree.StaticMemberExpression;
};

function isExpressionNode(node: ESTree.Expression | ESTree.PrivateIdentifier): node is ESTree.Expression {
	return node.type !== "PrivateIdentifier";
}

function areEquivalentTargets(left: ESTree.Expression, right: ESTree.Expression, sourceCode: SourceCode): boolean {
	if (left.type !== right.type) return false;

	switch (left.type) {
		case "CallExpression":
			return right.type === "CallExpression" && sourceCode.getText(left) === sourceCode.getText(right);

		case "Identifier":
			return right.type === "Identifier" && left.name === right.name;

		case "Literal":
			return right.type === "Literal" && left.value === right.value && left.raw === right.raw;

		case "MemberExpression": {
			if (right.type !== "MemberExpression") return false;
			if (left.computed !== right.computed || left.optional !== right.optional) return false;
			if (!areEquivalentTargets(left.object, right.object, sourceCode)) return false;

			if (left.computed) {
				if (!isExpressionNode(left.property) || !isExpressionNode(right.property)) return false;
				return areEquivalentTargets(left.property, right.property, sourceCode);
			}

			if (left.property.type === "PrivateIdentifier" || right.property.type === "PrivateIdentifier") {
				return (
					left.property.type === "PrivateIdentifier" &&
					right.property.type === "PrivateIdentifier" &&
					left.property.name === right.property.name
				);
			}

			return right.property.type === "Identifier" && left.property.name === right.property.name;
		}

		case "Super":
			return right.type === "Super";

		case "ThisExpression":
			return right.type === "ThisExpression";

		default:
			return false;
	}
}

function isSafeMemberKey(node: ESTree.Expression): boolean {
	switch (node.type) {
		case "Identifier":
		case "Literal":
		case "ThisExpression":
			return true;

		case "MemberExpression": {
			if (node.optional || !isSafeFixTarget(node.object)) return false;
			if (node.computed) {
				return isExpressionNode(node.property) ? isSafeMemberKey(node.property) : false;
			}
			return node.property.type === "Identifier" || node.property.type === "PrivateIdentifier";
		}

		default:
			return false;
	}
}

function isSafeFixTarget(node: ESTree.Expression): boolean {
	switch (node.type) {
		case "Identifier":
		case "ThisExpression":
			return true;

		case "MemberExpression": {
			if (node.optional || !isSafeFixTarget(node.object)) return false;
			if (node.computed) {
				return isExpressionNode(node.property) ? isSafeMemberKey(node.property) : false;
			}
			return node.property.type === "Identifier" || node.property.type === "PrivateIdentifier";
		}

		default:
			return false;
	}
}

function isSizeCall(node: ESTree.Expression): node is SizeCallExpression {
	if (node.type !== "CallExpression") return false;
	if (node.optional) return false;
	if (node.arguments.length > 0) return false;
	if (node.callee.type !== "MemberExpression") return false;
	if (node.callee.optional) return false;
	if (node.callee.computed) return false;
	if (node.callee.property.type !== "Identifier") return false;
	return node.callee.property.name === "size";
}

function isAllowAutofixOption(value: unknown): value is { readonly allowAutofix?: boolean } {
	if (typeof value !== "object" || value === null) return false;
	if (!("allowAutofix" in value)) return true;
	return value.allowAutofix === undefined || typeof value.allowAutofix === "boolean";
}

const noArraySizeAssignment = defineRule({
	create(context): Visitor {
		const [options] = context.options;
		const allowAutofix = isAllowAutofixOption(options) && options.allowAutofix === true;
		const { sourceCode } = context;

		return {
			AssignmentExpression(node): void {
				if (node.operator !== "=" || node.left.type !== "MemberExpression" || !node.left.computed) return;
				if (!isSizeCall(node.left.property)) return;
				if (!areEquivalentTargets(node.left.object, node.left.property.callee.object, sourceCode)) {
					return;
				}

				const expressionStatement = node.parent.type === "ExpressionStatement" ? node.parent : undefined;
				const shouldAutofix =
					allowAutofix && expressionStatement !== undefined && isSafeFixTarget(node.left.object);

				if (!shouldAutofix) {
					context.report({
						messageId: "usePush",
						node,
					});
					return;
				}

				const targetText = sourceCode.getText(node.left.object);
				const rightText = sourceCode.getText(node.right);

				context.report({
					fix(fixer) {
						return fixer.replaceText(expressionStatement, `${targetText}.push(${rightText});`);
					},
					messageId: "usePush",
					node,
				});
			},
		} satisfies Visitor;
	},
	meta: {
		docs: {
			description:
				"Disallow array append assignments using array[array.size()] = value and prefer push-based appends.",
		},
		fixable: "code",
		messages: {
			usePush: "Do not append with array[array.size()] = value. Use array.push(value) instead.",
		},
		schema: [
			{
				additionalProperties: false,
				properties: {
					allowAutofix: {
						default: false,
						type: "boolean",
					},
				},
				type: "object",
			},
		],
		type: "problem",
	},
});

export default noArraySizeAssignment;
