import { walkAstSlop } from "@oxlint-utilities/react-hook-utilities";
import { defineRule } from "oxlint-plugin-utilities";

import type { ESTree, Visitor } from "oxlint-plugin-utilities";

type MessageId =
	| "asyncIifeInConstructor"
	| "awaitInConstructor"
	| "orphanedPromise"
	| "promiseChainInConstructor"
	| "unhandledAsyncCall";

interface ConstructorViolation {
	readonly data?: Record<string, string>;
	readonly messageId: MessageId;
	readonly node: ESTree.Node;
}

const PROMISE_CHAIN_METHODS = new Set(["catch", "finally", "then"]);

function getAsyncMethodNames(classBody: ESTree.ClassBody): ReadonlySet<string> {
	const asyncMethods = new Set<string>();

	for (const element of classBody.body) {
		if (element.type !== "MethodDefinition" || element.kind !== "method" || !element.value.async) continue;
		if (element.key.type !== "Identifier") continue;
		asyncMethods.add(element.key.name);
	}

	return asyncMethods;
}

function isPromiseChainCall(node: ESTree.CallExpression): boolean {
	if (node.callee.type !== "MemberExpression" || node.callee.property.type !== "Identifier") return false;
	return PROMISE_CHAIN_METHODS.has(node.callee.property.name);
}

function isAsyncIife({ callee }: ESTree.CallExpression): boolean {
	return (callee.type === "ArrowFunctionExpression" || callee.type === "FunctionExpression") && callee.async;
}

function getThisAsyncMethodName(
	{ callee }: ESTree.CallExpression,
	asyncMethods: ReadonlySet<string>,
): string | undefined {
	if (
		callee.type !== "MemberExpression" ||
		callee.object.type !== "ThisExpression" ||
		callee.property.type !== "Identifier"
	) {
		return undefined;
	}
	return asyncMethods.has(callee.property.name) ? callee.property.name : undefined;
}

function isAssignedToThisProperty(node: ESTree.Node): boolean {
	const { parent } = node;
	if (parent?.type !== "AssignmentExpression" || parent.right !== node) return false;
	return parent.left.type === "MemberExpression" && parent.left.object.type === "ThisExpression";
}

function getLocalVariableAssignment(node: ESTree.Node): string | undefined {
	const { parent } = node;
	if (parent?.type !== "VariableDeclarator" || parent.init !== node) return undefined;
	return parent.id.type === "Identifier" ? parent.id.name : undefined;
}

function isNonIifeFunction(node: ESTree.Node): boolean {
	if (node.type !== "ArrowFunctionExpression" && node.type !== "FunctionExpression") return false;
	return node.parent.type !== "CallExpression" || node.parent.callee !== node;
}

function isInsideSkippedFunction(node: ESTree.Node, constructorBody: ESTree.BlockStatement): boolean {
	let current: ESTree.Node = node;

	while (current !== constructorBody) {
		const { parent } = current;
		if (parent === null) return false;
		if (isNonIifeFunction(parent)) return true;
		current = parent;
	}

	return false;
}

function getAsyncMethodViolation(
	node: ESTree.CallExpression,
	asyncMethods: ReadonlySet<string>,
): ConstructorViolation | undefined {
	const methodName = getThisAsyncMethodName(node, asyncMethods);
	if (methodName === undefined || isAssignedToThisProperty(node)) return undefined;

	if (node.parent.type === "ExpressionStatement") {
		return {
			data: { methodName },
			messageId: "unhandledAsyncCall",
			node,
		};
	}

	const variableName = getLocalVariableAssignment(node);
	return variableName === undefined
		? undefined
		: {
				data: { variableName },
				messageId: "orphanedPromise",
				node,
			};
}

function findConstructorViolations(
	constructorBody: ESTree.BlockStatement,
	asyncMethods: ReadonlySet<string>,
): ReadonlyArray<ConstructorViolation> {
	const violations = new Array<ConstructorViolation>();
	let size = 0;

	function handleChild(child: ESTree.Node): void {
		if (child !== constructorBody && isInsideSkippedFunction(child, constructorBody)) return;

		if (child.type === "AwaitExpression") {
			violations[size++] = { messageId: "awaitInConstructor", node: child };
			return;
		}

		if (child.type !== "CallExpression") return;

		if (isPromiseChainCall(child)) violations[size++] = { messageId: "promiseChainInConstructor", node: child };
		if (isAsyncIife(child)) violations[size++] = { messageId: "asyncIifeInConstructor", node: child };

		const asyncViolation = getAsyncMethodViolation(child, asyncMethods);
		if (asyncViolation !== undefined) violations[size++] = asyncViolation;
	}

	walkAstSlop(constructorBody, handleChild);
	return violations;
}

const noAsyncConstructor = defineRule({
	create(context): Visitor {
		function reportViolation(violation: ConstructorViolation): void {
			context.report({
				data: violation.data,
				messageId: violation.messageId,
				node: violation.node,
			});
		}

		return {
			"MethodDefinition[kind='constructor']": function (node: ESTree.MethodDefinition): void {
				if (
					node.value.type !== "FunctionExpression" ||
					node.value.body?.type !== "BlockStatement" ||
					node.parent.type !== "ClassBody"
				) {
					return;
				}

				const asyncMethods = getAsyncMethodNames(node.parent);
				const constructorBody = node.value.body;
				const violations = findConstructorViolations(constructorBody, asyncMethods);

				for (const violation of violations) reportViolation(violation);
			},
		} satisfies Visitor;
	},
	meta: {
		docs: {
			description:
				"Disallow asynchronous operations inside class constructors. Constructors return immediately, so async work causes race conditions, unhandled rejections, and incomplete object states.",
		},
		messages: {
			asyncIifeInConstructor:
				"Refactor this asynchronous operation outside of the constructor. Async IIFEs create unhandled promises and incomplete object state.",
			awaitInConstructor:
				"Refactor this asynchronous operation outside of the constructor. Using 'await' in a constructor causes the class to be instantiated before the async operation completes.",
			orphanedPromise:
				"Refactor this asynchronous operation outside of the constructor. Promise assigned to '{{variableName}}' is never consumed - errors will be silently swallowed.",
			promiseChainInConstructor:
				"Refactor this asynchronous operation outside of the constructor. Promise chains (.then/.catch/.finally) in constructors lead to race conditions.",
			unhandledAsyncCall:
				"Refactor this asynchronous operation outside of the constructor. Calling async method '{{methodName}}' without handling its result creates uncontrolled async behavior.",
		},
		schema: [] as const,
		type: "problem",
	},
});

export default noAsyncConstructor;
