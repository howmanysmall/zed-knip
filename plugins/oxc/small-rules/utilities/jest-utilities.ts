import { getMemberPropertyName } from "@oxlint-utilities/ast-utilities";
import { isFunctionNode } from "@oxlint-utilities/oxc-utilities";
import { walkAst } from "@oxlint-utilities/react-hook-utilities";

import type { CallbackFunction } from "@oxlint-types/missing-types";
import type { ESTree } from "oxlint-plugin-utilities";

export interface ExpectCallCount {
	readonly deterministic: number;
	readonly hasExpectInCallback: boolean;
	readonly hasExpectInLoop: boolean;
	readonly hasIndeterminate: boolean;
	readonly indeterminate: number;
}

interface ExpectContext {
	readonly hasCallback: boolean;
	readonly hasIndeterminate: boolean;
	readonly hasLoop: boolean;
}

function isTestIdentifier(node: ESTree.Node): node is ESTree.IdentifierName {
	return node.type === "Identifier" && (node.name === "it" || node.name === "test");
}

function isTestModifierCall(node: ESTree.CallExpression): boolean {
	if (node.callee.type !== "MemberExpression" || !isTestIdentifier(node.callee.object)) return false;

	const propertyName = getMemberPropertyName(node.callee);
	return propertyName === "only" || propertyName === "skip";
}

function isEachFactoryCall(node: ESTree.CallExpression): boolean {
	if (node.callee.type !== "MemberExpression" || !isTestIdentifier(node.callee.object)) return false;

	return getMemberPropertyName(node.callee) === "each";
}

function getLastCallbackArgument(node: ESTree.CallExpression): CallbackFunction | undefined {
	const lastArgument = node.arguments.at(-1);
	return lastArgument === undefined || !isFunctionNode(lastArgument) ? undefined : lastArgument;
}

function isIndeterminateLoopNode(node: ESTree.Node): boolean {
	return (
		node.type === "DoWhileStatement" ||
		node.type === "ForInStatement" ||
		node.type === "ForOfStatement" ||
		node.type === "ForStatement" ||
		node.type === "WhileStatement"
	);
}

function isSameNode(left: ESTree.Node, right: ESTree.Node): boolean {
	return left.type === right.type && left.range[0] === right.range[0] && left.range[1] === right.range[1];
}

function mergeExpectContext(left: ExpectContext, right: ExpectContext): ExpectContext {
	return {
		hasCallback: left.hasCallback || right.hasCallback,
		hasIndeterminate: left.hasIndeterminate || right.hasIndeterminate,
		hasLoop: left.hasLoop || right.hasLoop,
	};
}

function getExpectContext(currentParent: ESTree.Node, root: ESTree.Node): ExpectContext {
	if (isSameNode(currentParent, root)) {
		return {
			hasCallback: false,
			hasIndeterminate: false,
			hasLoop: false,
		};
	}

	const ownContext: ExpectContext = {
		hasCallback: isFunctionNode(currentParent),
		hasIndeterminate:
			isFunctionNode(currentParent) ||
			isIndeterminateLoopNode(currentParent) ||
			currentParent.type === "ConditionalExpression" ||
			currentParent.type === "IfStatement" ||
			currentParent.type === "SwitchCase" ||
			currentParent.type === "TryStatement",
		hasLoop: isIndeterminateLoopNode(currentParent),
	};

	if (currentParent.parent === null) return ownContext;
	return mergeExpectContext(ownContext, getExpectContext(currentParent.parent, root));
}

export function isTestCaseCall(node: ESTree.CallExpression): boolean {
	if (node.callee.type === "Identifier") return isTestIdentifier(node.callee);
	if (isTestModifierCall(node)) return true;
	if (node.callee.type === "CallExpression") return isEachFactoryCall(node.callee);
	return false;
}

export function getTestCallback(node: ESTree.CallExpression): CallbackFunction | undefined {
	return isTestCaseCall(node) ? getLastCallbackArgument(node) : undefined;
}

export function isExpectCall(
	node: ESTree.CallExpression,
	additionalAssertionFunctions: ReadonlyArray<string> = [],
): boolean {
	return (
		node.callee.type === "Identifier" &&
		(node.callee.name === "expect" || additionalAssertionFunctions.includes(node.callee.name))
	);
}

export function isExpectAssertionsCall({ callee }: ESTree.CallExpression): boolean {
	if (callee.type !== "MemberExpression" || callee.object.type !== "Identifier" || callee.object.name !== "expect") {
		return false;
	}
	return getMemberPropertyName(callee) === "assertions";
}

export function isExpectHasAssertionsCall({ callee }: ESTree.CallExpression): boolean {
	if (callee.type !== "MemberExpression" || callee.object.type !== "Identifier" || callee.object.name !== "expect") {
		return false;
	}
	return getMemberPropertyName(callee) === "hasAssertions";
}

export function countExpectCalls(
	body: ESTree.Node,
	additionalAssertionFunctions: ReadonlyArray<string> = [],
): ExpectCallCount {
	let deterministic = 0;
	let hasExpectInCallback = false;
	let hasExpectInLoop = false;
	let hasIndeterminate = false;
	let indeterminate = 0;

	walkAst(body, (child): void => {
		if (child.type !== "CallExpression" || !isExpectCall(child, additionalAssertionFunctions)) return;

		const expectContext = getExpectContext(child.parent, body);
		if (expectContext.hasLoop) hasExpectInLoop = true;
		if (expectContext.hasCallback) hasExpectInCallback = true;

		if (expectContext.hasIndeterminate) {
			hasIndeterminate = true;
			indeterminate += 1;
			return;
		}

		deterministic += 1;
	});

	return {
		deterministic,
		hasExpectInCallback,
		hasExpectInLoop,
		hasIndeterminate,
		indeterminate,
	};
}
