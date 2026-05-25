import { defineRule } from "oxlint-plugin-utilities";

import type { Scope, Visitor } from "oxlint-plugin-utilities";

const SCREAMING_SNAKE_CASE = /^[A-Z][A-Z0-9_]*$/v;

function isTopScope(scope: Scope): boolean {
	const { type } = scope;
	if (type === "module" || type === "global") return true;

	if (scope.upper?.type === "global") {
		const { block } = scope.upper;
		if (block.type === "Program" && block.sourceType === "script") return true;
	}

	return false;
}

const preferModuleScopeConstants = defineRule({
	create(context): Visitor {
		let inConstDeclaration = false;

		return {
			VariableDeclaration(node): void {
				inConstDeclaration = node.kind === "const";
			},
			"VariableDeclaration:exit": function (): void {
				inConstDeclaration = false;
			},
			VariableDeclarator(node): void {
				const { id } = node;
				if (id.type !== "Identifier" || !SCREAMING_SNAKE_CASE.test(id.name)) return;

				if (!inConstDeclaration) {
					context.report({
						messageId: "mustUseConst",
						node,
					});
					return;
				}

				const scope = context.sourceCode.getScope(node);
				if (isTopScope(scope)) return;

				context.report({
					messageId: "mustBeModuleScope",
					node,
				});
			},
		};
	},
	meta: {
		docs: {
			description:
				"Prefer that screaming snake case variables always be defined using `const`, and always appear at module scope.",
		},
		messages: {
			mustBeModuleScope:
				"You must place screaming snake case at module scope. If this is not meant to be a module-scoped variable, use camelcase instead.",
			mustUseConst:
				"You must use `const` when defining screaming snake case variables. If this is not a constant, use camelcase instead.",
		},
		schema: [] as const,
		type: "suggestion",
	},
});

export default preferModuleScopeConstants;
