import { getVariableByName, unwrapExpression } from "@oxlint-utilities/ast-utilities";

import type { ScopeVariable } from "@oxlint-utilities/ast-utilities";
import type { ESTree, Scope, SourceCode } from "oxlint-plugin-utilities";

export interface StaticExpressionOptions {
	readonly staticGlobalFactories: ReadonlySet<string>;
}

export const DEFAULT_STATIC_GLOBAL_FACTORIES: ReadonlyArray<string> = [
	"Axes",
	"BrickColor",
	"CFrame",
	"Color3",
	"ColorSequence",
	"ColorSequenceKeypoint",
	"DateTime",
	"Enum",
	"Faces",
	"NumberRange",
	"NumberSequence",
	"NumberSequenceKeypoint",
	"PathWaypoint",
	"PhysicalProperties",
	"Ray",
	"Rect",
	"Region3",
	"Region3int16",
	"TweenInfo",
	"UDim",
	"UDim2",
	"Vector2",
	"Vector3",
	"Vector3int16",
	"Vector3int32",
];

const STATIC_UNARY_OPERATORS = new Set(["!", "+", "-", "typeof", "void", "~"]);

const VALID_EXPRESSIONS = new Set<ESTree.Expression["type"]>([
	"ArrayExpression",
	"ArrowFunctionExpression",
	"AssignmentExpression",
	"AwaitExpression",
	"BinaryExpression",
	"CallExpression",
	"ChainExpression",
	"ClassExpression",
	"ConditionalExpression",
	"FunctionExpression",
	"Identifier",
	"ImportExpression",
	"Literal",
	"LogicalExpression",
	"MemberExpression",
	"MetaProperty",
	"NewExpression",
	"ObjectExpression",
	"ParenthesizedExpression",
	"SequenceExpression",
	"Super",
	"TaggedTemplateExpression",
	"TemplateLiteral",
	"ThisExpression",
	"TSAsExpression",
	"TSInstantiationExpression",
	"TSNonNullExpression",
	"TSSatisfiesExpression",
	"TSTypeAssertion",
	"UnaryExpression",
	"UpdateExpression",
	"YieldExpression",
]);

function isExpression(node: ESTree.Node): node is ESTree.Expression {
	return VALID_EXPRESSIONS.has(node.type);
}

export function isModuleLevelScope(scope: Scope): boolean {
	return scope.type === "module" || scope.type === "global";
}

export function isImportBinding(variable: ScopeVariable): boolean {
	for (const definition of variable.defs) {
		if (definition.type === "ImportBinding") return true;
	}
	return false;
}

function isVariableDefinition(definition: ScopeVariable["defs"][number]): boolean {
	return definition.type === "Variable";
}

export function getConstInitializer(definition: ScopeVariable["defs"][number]): ESTree.Expression | undefined {
	if (!isVariableDefinition(definition)) return undefined;

	const declarator = definition.node;
	if (declarator.type !== "VariableDeclarator") return undefined;

	const { parent } = declarator;
	if (parent.type !== "VariableDeclaration" || parent.kind !== "const") return undefined;

	return declarator.init ?? undefined;
}

export function getModuleConstInitializer(
	sourceCode: SourceCode,
	identifier: ESTree.IdentifierReference,
): ESTree.Expression | undefined {
	const variable = getVariableByName(sourceCode.getScope(identifier), identifier.name);
	if (variable === undefined || !isModuleLevelScope(variable.scope)) return undefined;

	for (const definition of variable.defs) {
		const initializer = getConstInitializer(definition);
		if (initializer !== undefined) return initializer;
	}

	return undefined;
}

export function getConstInitializerForIdentifier(
	sourceCode: SourceCode,
	identifier: ESTree.IdentifierReference,
): ESTree.Expression | undefined {
	const variable = getVariableByName(sourceCode.getScope(identifier), identifier.name);
	if (variable === undefined) return undefined;

	for (const definition of variable.defs) {
		const initializer = getConstInitializer(definition);
		if (initializer !== undefined) return initializer;
	}

	return undefined;
}

export function isExplicitUndefinedExpression(
	sourceCode: SourceCode,
	expression: ESTree.Expression,
	seen: Set<ESTree.Node>,
): boolean {
	const unwrapped = unwrapExpression(expression);
	if (seen.has(unwrapped)) return false;
	seen.add(unwrapped);

	if (
		(unwrapped.type === "Identifier" && unwrapped.name === "undefined") ||
		(unwrapped.type === "UnaryExpression" && unwrapped.operator === "void")
	) {
		return true;
	}
	if (unwrapped.type !== "Identifier") return false;

	const initializer = getConstInitializerForIdentifier(sourceCode, unwrapped);
	return initializer === undefined ? false : isExplicitUndefinedExpression(sourceCode, initializer, seen);
}

function isStaticMemberProperty(
	sourceCode: SourceCode,
	property: ESTree.Expression | ESTree.IdentifierName | ESTree.PrivateIdentifier,
	seen: Set<ESTree.Node>,
	options: StaticExpressionOptions,
): boolean {
	if (property.type === "Identifier") return true;
	if (!isExpression(property)) return false;
	return isStaticExpression(sourceCode, property, seen, options);
}

function isStaticCallCallee(
	sourceCode: SourceCode,
	callee: ESTree.Expression,
	seen: Set<ESTree.Node>,
	options: StaticExpressionOptions,
): boolean {
	const unwrapped = unwrapExpression(callee);

	if (unwrapped.type === "Identifier") {
		return isStaticIdentifier(sourceCode, unwrapped, seen, options);
	}

	if (unwrapped.type !== "MemberExpression") return false;
	if (!isStaticExpression(sourceCode, unwrapped.object, seen, options)) return false;

	if (unwrapped.computed) {
		return isStaticExpression(sourceCode, unwrapped.property, seen, options);
	}

	return unwrapped.property.type === "Identifier";
}

function checkStaticCallOrNewExpression(
	sourceCode: SourceCode,
	parameters: ReadonlyArray<ESTree.CallExpression["arguments"][number]>,
	callee: ESTree.Expression,
	seen: Set<ESTree.Node>,
	options: StaticExpressionOptions,
): boolean {
	if (!isStaticCallCallee(sourceCode, callee, seen, options)) return false;

	return parameters.every(
		(argument) => argument.type !== "SpreadElement" && isStaticExpression(sourceCode, argument, seen, options),
	);
}

export function isStaticExpression(
	sourceCode: SourceCode,
	expression: ESTree.Expression,
	seen: Set<ESTree.Node>,
	options: StaticExpressionOptions,
): boolean {
	const unwrapped = unwrapExpression(expression);
	if (seen.has(unwrapped)) return true;
	seen.add(unwrapped);

	switch (unwrapped.type) {
		case "ArrayExpression":
			return isStaticArrayExpression(sourceCode, unwrapped, seen, options);

		case "BinaryExpression":
		case "LogicalExpression": {
			if (!isExpression(unwrapped.left)) return false;
			return (
				isStaticExpression(sourceCode, unwrapped.left, seen, options) &&
				isStaticExpression(sourceCode, unwrapped.right, seen, options)
			);
		}

		case "CallExpression":
			return checkStaticCallOrNewExpression(sourceCode, unwrapped.arguments, unwrapped.callee, seen, options);

		case "ChainExpression":
			return isStaticExpression(sourceCode, unwrapped.expression, seen, options);

		case "ConditionalExpression": {
			return (
				isStaticExpression(sourceCode, unwrapped.test, seen, options) &&
				isStaticExpression(sourceCode, unwrapped.consequent, seen, options) &&
				isStaticExpression(sourceCode, unwrapped.alternate, seen, options)
			);
		}

		case "Identifier":
			return isStaticIdentifier(sourceCode, unwrapped, seen, options);

		case "Literal":
			return true;

		case "MemberExpression": {
			return (
				isStaticExpression(sourceCode, unwrapped.object, seen, options) &&
				(!unwrapped.computed || isStaticMemberProperty(sourceCode, unwrapped.property, seen, options))
			);
		}

		case "NewExpression":
			return checkStaticCallOrNewExpression(sourceCode, unwrapped.arguments, unwrapped.callee, seen, options);

		case "ObjectExpression":
			return isStaticObjectExpression(sourceCode, unwrapped, seen, options);

		case "SequenceExpression": {
			return (
				unwrapped.expressions.length > 0 &&
				unwrapped.expressions.every((expr) => isStaticExpression(sourceCode, expr, seen, options))
			);
		}

		case "TemplateLiteral":
			return unwrapped.expressions.length === 0;

		case "UnaryExpression": {
			return (
				STATIC_UNARY_OPERATORS.has(unwrapped.operator) &&
				isStaticExpression(sourceCode, unwrapped.argument, seen, options)
			);
		}

		default:
			return false;
	}
}

export function isStaticIdentifier(
	sourceCode: SourceCode,
	identifier: ESTree.IdentifierReference,
	seen: Set<ESTree.Node>,
	options: StaticExpressionOptions,
): boolean {
	const variable = getVariableByName(sourceCode.getScope(identifier), identifier.name);
	if (variable === undefined) return options.staticGlobalFactories.has(identifier.name);
	if (!isModuleLevelScope(variable.scope)) return false;
	if (isImportBinding(variable)) return true;

	for (const definition of variable.defs) {
		const initializer = getConstInitializer(definition);
		if (initializer === undefined) continue;
		if (isStaticExpression(sourceCode, initializer, seen, options)) return true;
	}

	return false;
}

function isExpressionKey(key: ESTree.ObjectProperty["key"]): key is ESTree.Expression {
	return key.type !== "PrivateIdentifier" && key.type !== "Identifier";
}

export function isStaticObjectExpression(
	sourceCode: SourceCode,
	objectExpr: ESTree.ObjectExpression,
	seen: Set<ESTree.Node>,
	options: StaticExpressionOptions,
): boolean {
	for (const property of objectExpr.properties) {
		if (property.type !== "Property") return false;
		if (property.kind !== "init") return false;

		if (
			(property.computed &&
				isExpressionKey(property.key) &&
				!isStaticExpression(sourceCode, property.key, seen, options)) ||
			!isStaticExpression(sourceCode, property.value, seen, options)
		) {
			return false;
		}
	}
	return true;
}

export function isStaticArrayExpression(
	sourceCode: SourceCode,
	{ elements }: ESTree.ArrayExpression,
	seen: Set<ESTree.Node>,
	options: StaticExpressionOptions,
): boolean {
	for (const element of elements) {
		if (element === null) return false;
		if (element.type === "SpreadElement" || !isStaticExpression(sourceCode, element, seen, options)) return false;
	}
	return true;
}
