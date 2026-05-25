import { isLiteralNode } from "@oxlint-utilities/oxc-utilities";
import { defineRule } from "oxlint-plugin-utilities";

import type { ESTree, Visitor } from "oxlint-plugin-utilities";

function isSimpleLiteralProperty(property: ESTree.ObjectProperty): boolean {
	return !property.computed && isSimpleLiteral(property.value);
}

function isSimpleLiteral(node: ESTree.Expression | undefined): boolean {
	if (node === undefined) return false;

	switch (node.type) {
		case "ArrayExpression": {
			return node.elements.every((element) => {
				if (element === null) return true;
				if (element.type === "SpreadElement") return false;
				return isSimpleLiteral(element);
			});
		}

		case "CallExpression":
			return node.callee.type === "MemberExpression" && isSimpleLiteral(node.callee.object);

		case "Literal":
			return true;

		case "MemberExpression":
			return isSimpleLiteral(node.object);

		case "ObjectExpression": {
			return node.properties.every((property) => {
				if (property.type === "SpreadElement") return false;
				return isSimpleLiteralProperty(property);
			});
		}

		default:
			return false;
	}
}

function isStaticMemberExpression(node: ESTree.MemberExpression): boolean {
	let current: ESTree.Expression = node;
	while (current.type === "MemberExpression") {
		if (current.computed && !isLiteralNode(current.property)) return false;
		current = current.object;
	}
	return true;
}

function isConstructor(node: ESTree.ClassElement): node is ESTree.MethodDefinition {
	return (
		node.type === "MethodDefinition" &&
		node.kind === "constructor" &&
		node.key.type === "Identifier" &&
		node.key.name === "constructor"
	);
}

const preferClassProperties = defineRule({
	create(context): Visitor {
		// eslint-disable-next-line ts/no-useless-default-assignment -- wrong.
		const [mode = "always"] = context.options;

		if (mode === "never") {
			return {
				PropertyDefinition(node): void {
					if (node.static) return;

					context.report({
						messageId: "unexpectedClassProperty",
						node,
					});
				},
			} satisfies Visitor;
		}

		return {
			ClassDeclaration(node): void {
				for (const member of node.body.body) {
					if (!isConstructor(member) || member.value.body === null) continue;

					for (const statement of member.value.body.body) {
						if (statement.type !== "ExpressionStatement") continue;

						const { expression } = statement;
						if (expression.type !== "AssignmentExpression") continue;

						const { left } = expression;
						if (left.type !== "MemberExpression" || left.object.type !== "ThisExpression") continue;

						const { property } = left;
						if (
							(property.type === "Identifier" || isLiteralNode(property)) &&
							isSimpleLiteral(expression.right) &&
							isStaticMemberExpression(left)
						) {
							context.report({
								messageId: "unexpectedAssignment",
								node: expression,
							});
						}
					}
				}
			},
			ClassExpression(node): void {
				for (const member of node.body.body) {
					if (!isConstructor(member) || member.value.body === null) continue;

					for (const statement of member.value.body.body) {
						if (statement.type !== "ExpressionStatement") continue;

						const { expression } = statement;
						if (expression.type !== "AssignmentExpression") continue;

						const { left } = expression;
						if (left.type !== "MemberExpression" || left.object.type !== "ThisExpression") continue;

						const { property } = left;
						if (
							(property.type === "Identifier" || isLiteralNode(property)) &&
							isSimpleLiteral(expression.right) &&
							isStaticMemberExpression(left)
						) {
							context.report({
								messageId: "unexpectedAssignment",
								node: expression,
							});
						}
					}
				}
			},
		} satisfies Visitor;
	},
	meta: {
		docs: {
			description: "Prefer class properties to assignment of literals in constructors.",
		},
		messages: {
			unexpectedAssignment:
				"Constructor assigns a literal value to this.property. Literals are static and known at class definition time. Move to a class property declaration: propertyName = value; at class level. This clarifies intent and reduces constructor complexity.",
			unexpectedClassProperty:
				"Class property declarations are disabled by rule configuration (mode: 'never'). Move initialization into the constructor: this.propertyName = value; inside constructor().",
		},
		schema: [
			{
				enum: ["always", "never"],
				type: "string",
			},
		],
		type: "suggestion",
	},
});

export default preferClassProperties;
