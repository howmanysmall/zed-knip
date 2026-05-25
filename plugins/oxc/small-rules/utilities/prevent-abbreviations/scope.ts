import {
	hasName,
	isAssignmentExpressionNode,
	isAssignmentPattern,
	isClassNode,
	isExportNamedDeclarationNode,
	isExportSpecifierNode,
	isFunctionDeclarationNode,
	isIdentifierName,
	isImportDeclaration,
	isImportDefaultSpecifierNode,
	isImportNamespaceSpecifierNode,
	isImportSpecifierNode,
	isJsxIdentifier,
	isMemberExpression,
	isMethodDefinitionNode,
	isObjectExpressionNode,
	isPropertyDefinitionNode,
	isPropertyNode,
	isStaticRequire,
	isStringLiteral,
	isTsTypeAliasDeclarationNode,
	isVariableDeclarationNode,
	isVariableDeclarator,
} from "@oxlint-utilities/oxc-utilities";

import { isValidIdentifier } from "./identifier";

import type { Definition, ESTree, Fix, Fixer, Scope, Variable } from "oxlint-plugin-utilities";

import type { BroadIdentifier, ImportCheckOption, IsSafe, NodeRange, VariableLike } from "./types";

export function getScopes(scope: Scope): Array<Scope> {
	const result = [scope];
	let size = 1;
	for (const child of scope.childScopes) {
		const childScopes = getScopes(child);
		for (const childScope of childScopes) result[size++] = childScope;
	}
	return result;
}

function resolveVariableName(name: string, scope: null | Scope): undefined | Variable {
	let currentScope = scope;
	while (currentScope !== null) {
		const variable = currentScope.set.get(name);
		if (variable !== undefined) return variable;
		currentScope = currentScope.upper;
	}
	return undefined;
}

function isSafeName(name: string, scopes: ReadonlyArray<Scope>): boolean {
	return !scopes.some((scope) => resolveVariableName(name, scope) !== undefined);
}

export function getAvailableVariableName(
	name: string,
	scopes: ReadonlyArray<Scope>,
	isSafe: IsSafe = () => true,
): string | undefined {
	let candidate = name;
	if (!isValidIdentifier(candidate)) {
		candidate = `${candidate}_`;
		if (!isValidIdentifier(candidate)) return undefined;
	}

	while (!isSafeName(candidate, scopes) || !isSafe(candidate, scopes)) candidate = `${candidate}_`;
	return candidate;
}

export function getVariableIdentifiers(variable: VariableLike): ReadonlyArray<BroadIdentifier> {
	const identifiers = new Set<BroadIdentifier>();
	for (const identifier of variable.identifiers) identifiers.add(identifier);
	for (const { identifier } of variable.references) identifiers.add(identifier);
	return [...identifiers];
}

function hasSameRange(node1: NodeRange, node2: NodeRange): boolean {
	return node1.range[0] === node2.range[0] && node1.range[1] === node2.range[1];
}

export function isShorthandImportLocal(node: ESTree.BindingIdentifier | ESTree.IdentifierName): boolean {
	const { parent } = node;
	if (!isImportSpecifierNode(parent) || parent.local !== node) return false;
	return hasSameRange(parent.local, parent.imported);
}

export function isShorthandExportLocal(node: ESTree.BindingIdentifier | ESTree.IdentifierName): boolean {
	const { parent } = node;
	if (!isExportSpecifierNode(parent) || parent.local !== node) return false;
	return hasSameRange(parent.local, parent.exported);
}

export function isShorthandPropertyValue(identifier: BroadIdentifier): boolean {
	if (!hasName(identifier)) return false;
	const { parent } = identifier;
	return isPropertyNode(parent) && parent.shorthand && parent.value === identifier;
}

export function isShorthandPropertyAssignmentPatternLeft(identifier: BroadIdentifier): boolean {
	if (!hasName(identifier)) return false;

	const { parent } = identifier;
	if (!isAssignmentPattern(parent) || parent.left !== identifier) return false;

	const property = parent.parent;
	if (!isPropertyNode(property)) return false;
	return property.shorthand;
}

export function isDefaultOrNamespaceImportName(identifier: BroadIdentifier): boolean {
	if (!hasName(identifier)) return false;

	const { parent } = identifier;
	if (
		(isImportDefaultSpecifierNode(parent) && parent.local === identifier) ||
		(isImportNamespaceSpecifierNode(parent) && parent.local === identifier)
	) {
		return true;
	}

	if (isImportSpecifierNode(parent) && parent.local === identifier) {
		const { imported } = parent;
		if (isIdentifierName(imported) && imported.name === "default") return true;
	}

	return (
		isVariableDeclarator(parent) && parent.id === identifier && parent.init !== null && isStaticRequire(parent.init)
	);
}

export function isExportedIdentifier(identifier: BroadIdentifier): boolean {
	if (!hasName(identifier)) return false;
	const { parent } = identifier;

	if (isVariableDeclarator(parent) && parent.id === identifier) {
		const declaration = parent.parent;
		return isVariableDeclarationNode(declaration) ? isExportNamedDeclarationNode(declaration.parent) : false;
	}

	if (isFunctionDeclarationNode(parent) && parent.id === identifier) {
		return isExportNamedDeclarationNode(parent.parent);
	}

	if (isClassNode(parent) && parent.id === identifier) return isExportNamedDeclarationNode(parent.parent);
	if (isTsTypeAliasDeclarationNode(parent) && parent.id === identifier) {
		return isExportNamedDeclarationNode(parent.parent);
	}

	return false;
}

export function shouldFix(variable: VariableLike): boolean {
	return getVariableIdentifiers(variable).every(
		(identifier) => !isExportedIdentifier(identifier) && !isJsxIdentifier(identifier),
	);
}

export function replaceReferenceIdentifier(
	identifier: BroadIdentifier,
	replacement: string,
	fixer: Fixer,
): Fix | undefined {
	if (!hasName(identifier)) return undefined;

	if (isShorthandPropertyValue(identifier) || isShorthandPropertyAssignmentPatternLeft(identifier)) {
		return fixer.replaceText(identifier, `${identifier.name}: ${replacement}`);
	}

	if (isShorthandImportLocal(identifier)) {
		return fixer.replaceText(identifier, `${identifier.name} as ${replacement}`);
	}

	if (isShorthandExportLocal(identifier)) {
		return fixer.replaceText(identifier, `${replacement} as ${identifier.name}`);
	}

	return fixer.replaceText(identifier, replacement);
}

export function renameVariable(variable: VariableLike, replacement: string, fixer: Fixer): Array<Fix> {
	const fixes = new Array<Fix>();
	let size = 0;
	for (const identifier of getVariableIdentifiers(variable)) {
		const fix = replaceReferenceIdentifier(identifier, replacement, fixer);
		if (fix !== undefined) fixes[size++] = fix;
	}
	return fixes;
}

export function shouldReportIdentifierAsProperty(identifier: BroadIdentifier): boolean {
	if (!hasName(identifier)) return false;
	const { parent } = identifier;
	if (isMemberExpression(parent) && parent.property === identifier && !parent.computed) {
		const parentParent = parent.parent;
		if (isAssignmentExpressionNode(parentParent) && parentParent.left === parent) return true;
	}

	if (
		isPropertyNode(parent) &&
		parent.key === identifier &&
		!parent.computed &&
		!parent.shorthand &&
		isObjectExpressionNode(parent.parent)
	) {
		return true;
	}

	if (isExportSpecifierNode(parent) && parent.exported === identifier && parent.local !== identifier) return true;

	return (
		(isMethodDefinitionNode(parent) || isPropertyDefinitionNode(parent)) &&
		parent.key === identifier &&
		!parent.computed
	);
}

export function isObjectPropertyKey(identifier: BroadIdentifier): boolean {
	if (!hasName(identifier)) return false;
	const { parent } = identifier;
	return (
		isPropertyNode(parent) &&
		parent.key === identifier &&
		!parent.computed &&
		!parent.shorthand &&
		isObjectExpressionNode(parent.parent)
	);
}

function getImportSource(definition: Definition): string | undefined {
	if (definition.type === "ImportBinding") {
		const { parent } = definition;
		if (parent !== null && isImportDeclaration(parent) && isStringLiteral(parent.source)) {
			return parent.source.value;
		}
	}

	if (definition.type === "Variable") {
		const { node } = definition;
		if (isVariableDeclarator(node) && node.init !== null && isStaticRequire(node.init)) {
			const [argument] = node.init.arguments;
			if (argument !== undefined && isStringLiteral(argument)) return argument.value;
		}
	}

	return undefined;
}

function isInternalImport(definition: Definition): boolean {
	const source = getImportSource(definition);
	if (source === undefined) return false;
	return !source.includes("node_modules") && (source.startsWith(".") || source.startsWith("/"));
}

export function shouldCheckImport(option: ImportCheckOption, definition: Definition): boolean {
	if (option === false) return false;
	return option === "internal" ? isInternalImport(definition) : true;
}

export function isClassVariable(variable: Variable): boolean {
	if (variable.defs.length !== 1) return false;
	return variable.defs[0]?.type === "ClassName";
}
