import { isRecord } from "@oxlint-utilities/type-utilities";
import { defineRule } from "oxlint-plugin-utilities";

import type { ESTree, Visitor } from "oxlint-plugin-utilities";

const DEFAULT_MAXIMUM_STATEMENTS = 1;

function getMaximumStatements(value: unknown): number {
	if (!isRecord(value) || typeof value.maximumStatements !== "number") return DEFAULT_MAXIMUM_STATEMENTS;
	return value.maximumStatements;
}

function isLonelyIfStatement(statement: ESTree.Statement): statement is ESTree.IfStatement {
	return statement.type === "IfStatement" && statement.alternate === null;
}

function isOffendingConsequent(consequent: ESTree.IfStatement["consequent"], maximumStatements: number): boolean {
	return (
		(consequent.type === "ExpressionStatement" && maximumStatements === 0) ||
		(consequent.type === "BlockStatement" && consequent.body.length > maximumStatements)
	);
}

function canSimplifyConditionalBody(body: ESTree.BlockStatement, maximumStatements: number): boolean {
	if (body.body.length !== 1) return false;

	const [statement] = body.body;
	if (statement === undefined || !isLonelyIfStatement(statement)) return false;

	return isOffendingConsequent(statement.consequent, maximumStatements);
}

const preferEarlyReturn = defineRule({
	create(context): Visitor {
		const maximumStatements = getMaximumStatements(context.options[0]);

		function checkFunctionBody(body: ESTree.BlockStatement): void {
			if (!canSimplifyConditionalBody(body, maximumStatements)) return;
			context.report({
				messageId: "preferEarlyReturn",
				node: body,
			});
		}

		return {
			ArrowFunctionExpression(node): void {
				if (node.body.type === "BlockStatement") checkFunctionBody(node.body);
			},
			FunctionDeclaration(node): void {
				if (node.body !== null) checkFunctionBody(node.body);
			},
			FunctionExpression(node): void {
				if (node.body !== null) checkFunctionBody(node.body);
			},
		} satisfies Visitor;
	},
	meta: {
		docs: {
			description: "Prefer early returns over full-body conditional wrapping.",
			recommended: true,
		},
		messages: {
			preferEarlyReturn:
				"Function body is wrapped in a single conditional without an else branch. This increases nesting depth and cognitive load. Invert the condition and return early: if (!condition) return; then place the main logic at the top level.",
		},
		schema: [
			{
				additionalProperties: false,
				properties: {
					maximumStatements: {
						default: 1,
						minimum: 0,
						type: "number",
					},
				},
				type: "object",
			},
		],
		type: "suggestion",
	},
});

export default preferEarlyReturn;
