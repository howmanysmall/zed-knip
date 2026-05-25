import { isRecord, isStringArray, isStringRecord } from "@oxlint-utilities/type-utilities";
import { defineRule } from "oxlint-plugin-utilities";

import type { ESTree, Visitor } from "oxlint-plugin-utilities";

interface BannedTypeEntry {
	readonly originalName: string;
	readonly replacementName?: string | undefined;
}

const DEFAULT_BANNED_TYPES = new Map<string, BannedTypeEntry>([
	["omit", { originalName: "Omit", replacementName: "Except" }],
]);

function normalizeBannedTypes(rawOptions: unknown): ReadonlyMap<string, BannedTypeEntry> {
	const bannedTypes = new Map(DEFAULT_BANNED_TYPES);

	if (!isRecord(rawOptions) || !("bannedTypes" in rawOptions)) return bannedTypes;

	const { bannedTypes: configuredBannedTypes } = rawOptions;
	if (configuredBannedTypes === undefined) return bannedTypes;

	if (isStringArray(configuredBannedTypes)) {
		for (const typeName of configuredBannedTypes) {
			bannedTypes.set(typeName.toLowerCase(), { originalName: typeName, replacementName: undefined });
		}
		return bannedTypes;
	}

	if (isStringRecord(configuredBannedTypes)) {
		for (const [typeName, replacementName] of Object.entries(configuredBannedTypes)) {
			bannedTypes.set(typeName.toLowerCase(), { originalName: typeName, replacementName });
		}
	}

	return bannedTypes;
}

function getReferencedTypeName(typeNameNode: ESTree.TSTypeName): string | undefined {
	if (typeNameNode.type === "Identifier") return typeNameNode.name;
	if (typeNameNode.type === "TSQualifiedName") return typeNameNode.right.name;
	return undefined;
}

const banTypes = defineRule({
	create(context): Visitor {
		const [rawOptions] = context.options;

		const bannedTypes = normalizeBannedTypes(rawOptions);
		if (bannedTypes.size === 0) return {} satisfies Visitor;

		return {
			TSTypeReference(node): void {
				const referencedTypeName = getReferencedTypeName(node.typeName);
				if (referencedTypeName === undefined) return;

				const bannedTypeEntry = bannedTypes.get(referencedTypeName.toLowerCase());
				if (bannedTypeEntry === undefined) return;

				if (bannedTypeEntry.replacementName !== undefined && bannedTypeEntry.replacementName !== "") {
					context.report({
						data: {
							replacementName: bannedTypeEntry.replacementName,
							typeName: bannedTypeEntry.originalName,
						},
						messageId: "bannedTypeWithReplacement",
						node: node.typeName,
					});
					return;
				}

				context.report({
					data: { typeName: bannedTypeEntry.originalName },
					messageId: "bannedType",
					node: node.typeName,
				});
			},
		} satisfies Visitor;
	},
	meta: {
		docs: {
			description: "Ban configured TypeScript utility types, defaulting to Omit in favor of Except.",
		},
		messages: {
			bannedType:
				"Type '{{typeName}}' is banned by project configuration. Use the project-preferred alternative for this type.",
			bannedTypeWithReplacement: "Type '{{typeName}}' is banned. Use '{{replacementName}}' instead.",
		},
		schema: [
			{
				additionalProperties: false,
				properties: {
					bannedTypes: {
						description:
							"Array of banned type names or an object mapping banned type names to preferred replacement names.",
						oneOf: [
							{
								items: { type: "string" },
								type: "array",
							},
							{
								additionalProperties: { type: "string" },
								type: "object",
							},
						],
					},
				},
				type: "object",
			},
		],
		type: "problem",
	},
});

export default banTypes;
