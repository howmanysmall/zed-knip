// oxlint-disable typescript/no-unnecessary-condition
import { getMemberPropertyName, unwrapExpression } from "@oxlint-utilities/ast-utilities";
import { isNonEmptyString } from "@oxlint-utilities/type-utilities";
import { defineRule } from "oxlint-plugin-utilities";

import type { ESTree, Visitor } from "oxlint-plugin-utilities";

export interface NoConstantConditionWithBreakOptions {
	readonly loopExitCalls?: ReadonlyArray<string>;
}

interface ConstantValueResult {
	readonly constant: boolean;
	readonly value?: unknown;
}

interface ConstantBooleanResult {
	readonly constant: boolean;
	readonly value?: boolean;
}

function toConstantValue(value: unknown): ConstantValueResult {
	return { constant: true, value };
}

function toNonConstantValue(): ConstantValueResult {
	return { constant: false };
}

function toConstantBoolean(value: boolean): ConstantBooleanResult {
	return { constant: true, value };
}

function toNonConstantBoolean(): ConstantBooleanResult {
	return { constant: false };
}

function normalizeLoopExitCalls(options: NoConstantConditionWithBreakOptions | undefined): ReadonlySet<string> {
	const loopExitCalls = new Set<string>();
	if (!options?.loopExitCalls) return loopExitCalls;

	for (const loopExitCall of options.loopExitCalls) {
		if (isNonEmptyString(loopExitCall)) loopExitCalls.add(loopExitCall);
	}

	return loopExitCalls;
}

function getNodePath(node: ESTree.Expression): string | undefined {
	const unwrapped = unwrapExpression(node);

	if (unwrapped.type === "Identifier") return unwrapped.name;
	if (unwrapped.type !== "MemberExpression") return undefined;

	const objectPath = getNodePath(unwrapped.object);
	if (objectPath === undefined || objectPath.length === 0) return undefined;

	const propertyName = getMemberPropertyName(unwrapped);
	if (propertyName === undefined || propertyName.length === 0) return undefined;

	return `${objectPath}.${propertyName}`;
}

function isConfiguredLoopExitCall(callExpression: ESTree.CallExpression, loopExitCalls: ReadonlySet<string>): boolean {
	if (loopExitCalls.size === 0) return false;

	const calleePath = getNodePath(callExpression.callee);
	if (calleePath === undefined || calleePath.length === 0) return false;

	return loopExitCalls.has(calleePath);
}

function expressionContainsConfiguredLoopExit(
	expression: ESTree.Expression,
	loopExitCalls: ReadonlySet<string>,
): boolean {
	if (loopExitCalls.size === 0) return false;
	const unwrapped = unwrapExpression(expression);

	switch (unwrapped.type) {
		case "ArrayExpression": {
			for (const element of unwrapped.elements) {
				if (!element) continue;
				if (element.type === "SpreadElement") {
					if (expressionContainsConfiguredLoopExit(element.argument, loopExitCalls)) return true;
					continue;
				}

				if (expressionContainsConfiguredLoopExit(element, loopExitCalls)) return true;
			}

			return false;
		}

		case "ArrowFunctionExpression":
		case "ClassExpression":
		case "FunctionExpression":
			return false;

		case "AssignmentExpression":
			return expressionContainsConfiguredLoopExit(unwrapped.right, loopExitCalls);

		case "AwaitExpression":
			return expressionContainsConfiguredLoopExit(unwrapped.argument, loopExitCalls);

		case "BinaryExpression": {
			const { left } = unwrapped;
			if (left.type !== "PrivateIdentifier" && expressionContainsConfiguredLoopExit(left, loopExitCalls)) {
				return true;
			}
			return expressionContainsConfiguredLoopExit(unwrapped.right, loopExitCalls);
		}

		case "CallExpression": {
			if (
				isConfiguredLoopExitCall(unwrapped, loopExitCalls) ||
				expressionContainsConfiguredLoopExit(unwrapped.callee, loopExitCalls)
			) {
				return true;
			}

			for (const argument of unwrapped.arguments) {
				if (argument.type === "SpreadElement") {
					if (expressionContainsConfiguredLoopExit(argument.argument, loopExitCalls)) return true;
					continue;
				}

				if (expressionContainsConfiguredLoopExit(argument, loopExitCalls)) return true;
			}

			return false;
		}

		case "ConditionalExpression": {
			return (
				expressionContainsConfiguredLoopExit(unwrapped.test, loopExitCalls) ||
				expressionContainsConfiguredLoopExit(unwrapped.consequent, loopExitCalls) ||
				expressionContainsConfiguredLoopExit(unwrapped.alternate, loopExitCalls)
			);
		}

		case "LogicalExpression": {
			return (
				expressionContainsConfiguredLoopExit(unwrapped.left, loopExitCalls) ||
				expressionContainsConfiguredLoopExit(unwrapped.right, loopExitCalls)
			);
		}

		case "MemberExpression": {
			if (expressionContainsConfiguredLoopExit(unwrapped.object, loopExitCalls)) return true;

			if (unwrapped.computed) {
				return expressionContainsConfiguredLoopExit(unwrapped.property, loopExitCalls);
			}

			return false;
		}

		case "NewExpression": {
			if (expressionContainsConfiguredLoopExit(unwrapped.callee, loopExitCalls)) return true;

			for (const argument of unwrapped.arguments) {
				if (argument.type === "SpreadElement") {
					if (expressionContainsConfiguredLoopExit(argument.argument, loopExitCalls)) return true;
					continue;
				}

				if (expressionContainsConfiguredLoopExit(argument, loopExitCalls)) return true;
			}

			return false;
		}

		case "SequenceExpression":
			return unwrapped.expressions.some((part) => expressionContainsConfiguredLoopExit(part, loopExitCalls));

		case "TaggedTemplateExpression": {
			if (expressionContainsConfiguredLoopExit(unwrapped.tag, loopExitCalls)) return true;
			return unwrapped.quasi.expressions.some((part) =>
				expressionContainsConfiguredLoopExit(part, loopExitCalls),
			);
		}

		case "TemplateLiteral":
			return unwrapped.expressions.some((part) => expressionContainsConfiguredLoopExit(part, loopExitCalls));

		case "UnaryExpression":
		case "UpdateExpression":
			return expressionContainsConfiguredLoopExit(unwrapped.argument, loopExitCalls);

		case "YieldExpression":
			return unwrapped.argument ? expressionContainsConfiguredLoopExit(unwrapped.argument, loopExitCalls) : false;

		default:
			return false;
	}
}

function getConstantValue(expression: ESTree.Expression): ConstantValueResult {
	const unwrapped = unwrapExpression(expression);

	switch (unwrapped.type) {
		case "ArrayExpression":
			return toConstantValue(new Array<unknown>());

		case "ArrowFunctionExpression":
		case "ClassExpression":
		case "FunctionExpression":
			return toConstantValue(true);

		case "Identifier": {
			if (unwrapped.name === "undefined") return toConstantValue(undefined);
			if (unwrapped.name === "NaN") return toConstantValue(Number.NaN);
			if (unwrapped.name === "Infinity") return toConstantValue(Number.POSITIVE_INFINITY);
			return toNonConstantValue();
		}

		case "Literal":
			return toConstantValue(unwrapped.value);

		case "LogicalExpression": {
			const left = getConstantValue(unwrapped.left);
			if (!left.constant) return toNonConstantValue();

			if (unwrapped.operator === "&&") {
				if (left.value !== true) return toConstantValue(left.value);
				return getConstantValue(unwrapped.right);
			}

			if (unwrapped.operator === "||") {
				if (left.value === true) return toConstantValue(left.value);
				return getConstantValue(unwrapped.right);
			}

			if (left.value !== undefined) return toConstantValue(left.value);
			return getConstantValue(unwrapped.right);
		}

		case "ObjectExpression":
			return toConstantValue({});

		case "SequenceExpression": {
			const lastExpression = unwrapped.expressions.at(-1);
			if (!lastExpression) return toNonConstantValue();
			return getConstantValue(lastExpression);
		}

		case "TemplateLiteral": {
			if (unwrapped.expressions.length > 0) return toNonConstantValue();
			if (unwrapped.quasis.length === 0) return toConstantValue("");
			return toConstantValue(unwrapped.quasis[0]?.value.cooked ?? "");
		}

		case "UnaryExpression": {
			if (unwrapped.operator === "typeof") return toConstantValue("string");
			if (unwrapped.operator === "void") return toConstantValue(undefined);

			const argument = getConstantValue(unwrapped.argument);
			if (!argument.constant) return toNonConstantValue();

			// oxlint-disable-next-line typescript/strict-boolean-expressions
			if (unwrapped.operator === "!") return toConstantValue(!argument.value);
			if (unwrapped.operator === "+" && typeof argument.value === "number") {
				return toConstantValue(argument.value);
			}

			if (unwrapped.operator === "-" && typeof argument.value === "number") {
				return toConstantValue(-argument.value);
			}

			if (unwrapped.operator === "~" && typeof argument.value === "number") {
				return toConstantValue(~argument.value);
			}

			return toNonConstantValue();
		}

		default:
			return toNonConstantValue();
	}
}

function getConstantBoolean(expression: ESTree.Expression): ConstantBooleanResult {
	const unwrapped = unwrapExpression(expression);

	if (unwrapped.type === "ConditionalExpression") {
		const test = getConstantBoolean(unwrapped.test);
		if (test.constant) {
			return getConstantBoolean(test.value === true ? unwrapped.consequent : unwrapped.alternate);
		}

		const consequent = getConstantBoolean(unwrapped.consequent);
		const alternate = getConstantBoolean(unwrapped.alternate);
		if (consequent.constant && alternate.constant && consequent.value === alternate.value) return consequent;

		return toNonConstantBoolean();
	}

	if (unwrapped.type === "LogicalExpression") {
		const left = getConstantBoolean(unwrapped.left);
		if (!left.constant) return toNonConstantBoolean();

		if (unwrapped.operator === "&&") {
			// oxlint-disable-next-line typescript/strict-boolean-expressions
			if (!left.value) return toConstantBoolean(false);
			return getConstantBoolean(unwrapped.right);
		}

		if (unwrapped.operator === "||") {
			if (left.value === true) return toConstantBoolean(true);
			return getConstantBoolean(unwrapped.right);
		}

		const leftValue = getConstantValue(unwrapped.left);
		if (!leftValue.constant) return toNonConstantBoolean();
		if (leftValue.value !== undefined) {
			return toConstantBoolean(Boolean(leftValue.value));
		}

		return getConstantBoolean(unwrapped.right);
	}

	if (unwrapped.type === "SequenceExpression") {
		const lastExpression = unwrapped.expressions.at(-1);
		if (!lastExpression) return toNonConstantBoolean();
		return getConstantBoolean(lastExpression);
	}

	const value = getConstantValue(unwrapped);
	if (!value.constant) return toNonConstantBoolean();
	return toConstantBoolean(Boolean(value.value));
}

type LoopNode =
	| ESTree.DoWhileStatement
	| ESTree.ForInStatement
	| ESTree.ForOfStatement
	| ESTree.ForStatement
	| ESTree.WhileStatement;

const LOOP_TYPES = new Set(["DoWhileStatement", "ForInStatement", "ForOfStatement", "ForStatement", "WhileStatement"]);
function isLoopNode(node: ESTree.Node): node is LoopNode {
	return LOOP_TYPES.has(node.type);
}

const FUNCTION_BOUNDARY_TYPES = new Set(["ArrowFunctionExpression", "FunctionDeclaration", "FunctionExpression"]);
function isFunctionBoundary(node: ESTree.Node): boolean {
	return FUNCTION_BOUNDARY_TYPES.has(node.type);
}

function findLabeledStatementBody(labelName: string, startingNode: ESTree.Node): ESTree.Statement | undefined {
	let current: ESTree.Node | null = startingNode;

	while (current !== null) {
		if (current.type === "LabeledStatement" && current.label.name === labelName) return current.body;
		if (current.type === "Program") return undefined;
		current = current.parent;
	}

	return undefined;
}

function breaksTargetLoop(statement: ESTree.BreakStatement, loopNode: LoopNode): boolean {
	if (statement.label) {
		const target = findLabeledStatementBody(statement.label.name, statement.parent);
		return target === loopNode;
	}

	let current: ESTree.Node | null = statement.parent;
	while (current !== null) {
		if (current.type === "Program" || isFunctionBoundary(current) || current.type === "SwitchStatement") {
			return false;
		}
		if (isLoopNode(current)) return current === loopNode;
		current = current.parent;
	}

	return false;
}

function forStatementInitContainsConfiguredLoopExit(
	initialization: ESTree.ForStatement["init"],
	loopExitCalls: ReadonlySet<string>,
): boolean {
	if (!initialization) return false;

	if (initialization.type === "VariableDeclaration") {
		return initialization.declarations.some((declaration) =>
			declaration.init ? expressionContainsConfiguredLoopExit(declaration.init, loopExitCalls) : false,
		);
	}

	return expressionContainsConfiguredLoopExit(initialization, loopExitCalls);
}

function loopHeaderContainsConfiguredLoopExit(loopNode: LoopNode, loopExitCalls: ReadonlySet<string>): boolean {
	switch (loopNode.type) {
		case "DoWhileStatement":
		case "WhileStatement":
			return expressionContainsConfiguredLoopExit(loopNode.test, loopExitCalls);

		case "ForInStatement":
		case "ForOfStatement":
			return expressionContainsConfiguredLoopExit(loopNode.right, loopExitCalls);

		case "ForStatement": {
			if (forStatementInitContainsConfiguredLoopExit(loopNode.init, loopExitCalls)) return true;
			if (loopNode.test && expressionContainsConfiguredLoopExit(loopNode.test, loopExitCalls)) return true;
			if (loopNode.update && expressionContainsConfiguredLoopExit(loopNode.update, loopExitCalls)) return true;
			return false;
		}

		default:
			return false;
	}
}

function statementContainsLoopExit(
	statement: ESTree.Statement,
	loopNode: LoopNode,
	loopExitCalls: ReadonlySet<string>,
): boolean {
	switch (statement.type) {
		case "BlockStatement": {
			return statement.body.some((bodyStatement) =>
				statementContainsLoopExit(bodyStatement, loopNode, loopExitCalls),
			);
		}

		case "BreakStatement":
			return breaksTargetLoop(statement, loopNode);

		case "DoWhileStatement":
		case "WhileStatement": {
			if (expressionContainsConfiguredLoopExit(statement.test, loopExitCalls)) return true;
			return statementContainsLoopExit(statement.body, loopNode, loopExitCalls);
		}

		case "ExpressionStatement":
			return expressionContainsConfiguredLoopExit(statement.expression, loopExitCalls);

		case "ForInStatement":
		case "ForOfStatement": {
			if (expressionContainsConfiguredLoopExit(statement.right, loopExitCalls)) return true;
			return statementContainsLoopExit(statement.body, loopNode, loopExitCalls);
		}

		case "ForStatement": {
			if (forStatementInitContainsConfiguredLoopExit(statement.init, loopExitCalls)) return true;
			if (statement.test && expressionContainsConfiguredLoopExit(statement.test, loopExitCalls)) return true;
			if (statement.update && expressionContainsConfiguredLoopExit(statement.update, loopExitCalls)) return true;
			return statementContainsLoopExit(statement.body, loopNode, loopExitCalls);
		}

		case "IfStatement": {
			if (statementContainsLoopExit(statement.consequent, loopNode, loopExitCalls)) return true;
			return statement.alternate
				? statementContainsLoopExit(statement.alternate, loopNode, loopExitCalls)
				: false;
		}

		case "LabeledStatement":
			return statementContainsLoopExit(statement.body, loopNode, loopExitCalls);

		case "ReturnStatement":
			return true;

		case "SwitchStatement": {
			return statement.cases.some((switchCase) =>
				switchCase.consequent.some((consequent) =>
					statementContainsLoopExit(consequent, loopNode, loopExitCalls),
				),
			);
		}

		case "TryStatement": {
			if (statementContainsLoopExit(statement.block, loopNode, loopExitCalls)) return true;
			if (statement.handler && statementContainsLoopExit(statement.handler.body, loopNode, loopExitCalls)) {
				return true;
			}
			if (statement.finalizer && statementContainsLoopExit(statement.finalizer, loopNode, loopExitCalls)) {
				return true;
			}
			return false;
		}

		case "VariableDeclaration": {
			return statement.declarations.some((declaration) =>
				declaration.init ? expressionContainsConfiguredLoopExit(declaration.init, loopExitCalls) : false,
			);
		}

		case "WithStatement": {
			if (expressionContainsConfiguredLoopExit(statement.object, loopExitCalls)) return true;
			return statementContainsLoopExit(statement.body, loopNode, loopExitCalls);
		}

		default:
			return false;
	}
}

function shouldReportLoop(
	testResult: ConstantBooleanResult,
	loopNode: LoopNode,
	loopExitCalls: ReadonlySet<string>,
): boolean {
	if (!testResult.constant) return false;
	// oxlint-disable-next-line typescript/strict-boolean-expressions
	if (!testResult.value) return true;
	if (loopHeaderContainsConfiguredLoopExit(loopNode, loopExitCalls)) return false;
	return !statementContainsLoopExit(loopNode.body, loopNode, loopExitCalls);
}

const noConstantConditionWithBreak = defineRule({
	create(context): Visitor {
		const rawOptions = context.options?.[0];
		const loopExitCalls = normalizeLoopExitCalls(
			typeof rawOptions === "object" && rawOptions !== null
				? (rawOptions as NoConstantConditionWithBreakOptions)
				: undefined,
		);

		function reportConstantCondition(testExpression: ESTree.Expression): void {
			const testResult = getConstantBoolean(testExpression);
			if (!testResult.constant) return;

			context.report({
				messageId: "unexpected",
				node: testExpression,
			});
		}

		return {
			ConditionalExpression(node): void {
				reportConstantCondition(node.test);
			},
			DoWhileStatement(node): void {
				const testResult = getConstantBoolean(node.test);
				if (!shouldReportLoop(testResult, node, loopExitCalls)) return;

				context.report({
					messageId: "unexpected",
					node: node.test,
				});
			},
			ForStatement(node): void {
				if (!node.test) return;
				const testResult = getConstantBoolean(node.test);
				if (!shouldReportLoop(testResult, node, loopExitCalls)) return;

				context.report({
					messageId: "unexpected",
					node: node.test,
				});
			},
			IfStatement(node): void {
				reportConstantCondition(node.test);
			},
			WhileStatement(node): void {
				const testResult = getConstantBoolean(node.test);
				if (!shouldReportLoop(testResult, node, loopExitCalls)) return;

				context.report({
					messageId: "unexpected",
					node: node.test,
				});
			},
		} satisfies Visitor;
	},
	meta: {
		docs: {
			description:
				"Disallow constant conditions, but allow constant loops that include loop exits such as break, return, or configured calls.",
		},
		messages: {
			unexpected: "Unexpected constant condition.",
		},
		schema: [
			{
				additionalProperties: false,
				properties: {
					loopExitCalls: {
						items: {
							minLength: 1,
							type: "string",
						},
						type: "array",
					},
				},
				type: "object",
			},
		],
		type: "problem",
	},
});

export default noConstantConditionWithBreak;
