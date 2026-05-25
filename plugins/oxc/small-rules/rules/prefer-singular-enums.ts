import { defineRule } from "oxlint-plugin-utilities";

import type { Visitor } from "oxlint-plugin-utilities";

const IRREGULAR_PLURALS = new Set<string>([
	"alumni",
	"axes",
	"cacti",
	"children",
	"criteria",
	"data",
	"dice",
	"feet",
	"fungi",
	"geese",
	"indices",
	"matrices",
	"media",
	"men",
	"mice",
	"octopi",
	"people",
	"phenomena",
	"teeth",
	"vertices",
	"women",
]);

const SINGULAR_EXCEPTIONS = new Set<string>([
	"alias",
	"analysis",
	"axis",
	"basis",
	"business",
	"class",
	"crisis",
	"glass",
	"news",
	"series",
	"species",
	"status",
	"thesis",
]);

const PROGRAMMING_PLURALS = new Set<string>([
	"args",
	"components",
	"controllers",
	"dto",
	"dtos",
	"entries",
	"enums",
	"hooks",
	"items",
	"keys",
	"models",
	"options",
	"orders",
	"pages",
	"parameters",
	"params",
	"props",
	"repositories",
	"services",
	"settings",
	"types",
	"values",
	"vo",
	"vos",
]);

const TOKEN_PATTERN = /[A-Z]+(?![a-z])|[A-Z]?[a-z]+|\d+/gv;
const ACRONYM_PLURAL_PATTERN = /^[A-Z]{2,}[sS]?$/v;

interface AlphaToken {
	readonly lowercased: string;
	readonly original: string;
}

function tokenizeIdentifier(identifier: string): ReadonlyArray<string> {
	const tokens = new Array<string>();
	let size = 0;

	for (const part of identifier.split("_")) {
		const matches = part.match(TOKEN_PATTERN);
		if (matches === null) continue;
		for (const match of matches) tokens[size++] = match;
	}

	return tokens;
}

const IS_INTEGER_END = /^\d+$/v;

function getLastAlphaToken(tokens: ReadonlyArray<string>): AlphaToken | undefined {
	for (let index = tokens.length - 1; index >= 0; index -= 1) {
		const token = tokens[index];
		if (token === undefined || IS_INTEGER_END.test(token)) continue;
		return { lowercased: token.toLowerCase(), original: token };
	}

	return undefined;
}

const S_ENDING_REGEXP = /[sS]$/v;
const E_ENDING_REGEXP = /(?:ch|sh|x|z)es$/v;

function isPluralWord(word: string, original: string): boolean {
	if (IRREGULAR_PLURALS.has(word) || PROGRAMMING_PLURALS.has(word)) return true;
	if (SINGULAR_EXCEPTIONS.has(word)) return false;

	return (
		(ACRONYM_PLURAL_PATTERN.test(original) && S_ENDING_REGEXP.test(original)) ||
		word.endsWith("ies") ||
		word.endsWith("ves") ||
		E_ENDING_REGEXP.test(word) ||
		(word.endsWith("s") && !word.endsWith("ss") && !word.endsWith("us") && !word.endsWith("is"))
	);
}

function isPlural(identifier: string): boolean {
	if (ACRONYM_PLURAL_PATTERN.test(identifier) && S_ENDING_REGEXP.test(identifier)) return true;

	const lastToken = getLastAlphaToken(tokenizeIdentifier(identifier));
	return lastToken !== undefined && isPluralWord(lastToken.lowercased, lastToken.original);
}

const preferSingularEnums = defineRule({
	create(context): Visitor {
		return {
			TSEnumDeclaration({ id }): void {
				const { name } = id;
				if (!isPlural(name)) return;

				context.report({
					data: { name },
					messageId: "notSingular",
					node: id,
				});
			},
		} satisfies Visitor;
	},
	meta: {
		docs: {
			description: "Prefer singular naming for enums.",
			recommended: true,
		},
		messages: {
			notSingular: 'Enum name "{{name}}" should be singular.',
		},
		schema: [] as const,
		type: "suggestion",
	},
});

export default preferSingularEnums;
