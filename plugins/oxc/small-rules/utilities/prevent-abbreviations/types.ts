import type { Definition, ESTree, Reference, Scope } from "oxlint-plugin-utilities";

export type MessageIds = "replace" | "suggestion";
export type ImportCheckOption = "internal" | boolean;

export interface ShorthandMatcher {
	readonly original: string;
	readonly pattern: RegExp;
	readonly replacement: string;
	readonly replacementPatterns: ReadonlyArray<RegExp>;
}

export interface ShorthandMatch {
	readonly matchedWord: string;
	readonly replacement: string;
	readonly shorthand: string;
}

export interface ShorthandConfiguration {
	readonly exactMatchers: ReadonlyMap<string, string>;
	readonly ignoreExact: ReadonlySet<string>;
	readonly ignoreMatchers: ReadonlyArray<ShorthandMatcher>;
	readonly matchers: ReadonlyArray<ShorthandMatcher>;
}

export interface ShorthandReplacement {
	readonly matches: ReadonlyArray<ShorthandMatch>;
	readonly replaced: string;
}

export interface PreventAbbreviationsOptions {
	readonly allowList?: Record<string, boolean>;
	readonly allowPropertyAccess?: ReadonlyArray<string>;
	readonly checkDefaultAndNamespaceImports?: ImportCheckOption;
	readonly checkFilenames?: boolean;
	readonly checkProperties?: boolean;
	readonly checkShorthandImports?: ImportCheckOption;
	readonly checkShorthandProperties?: boolean;
	readonly checkVariables?: boolean;
	readonly extendDefaultAllowList?: boolean;
	readonly extendDefaultReplacements?: boolean;
	readonly ignore?: ReadonlyArray<RegExp | string>;
	readonly ignoreShorthands?: ReadonlyArray<string>;
	readonly replacements?: Record<string, false | Record<string, boolean>>;
	readonly shorthands?: Record<string, string>;
}

export interface PreparedOptions {
	readonly allowList: Map<string, boolean>;
	readonly allowPropertyAccess: ReadonlySet<string>;
	readonly checkDefaultAndNamespaceImports: ImportCheckOption;
	readonly checkFilenames: boolean;
	readonly checkProperties: boolean;
	readonly checkShorthandImports: ImportCheckOption;
	readonly checkShorthandProperties: boolean;
	readonly checkVariables: boolean;
	readonly ignore: ReadonlyArray<RegExp>;
	readonly replacements: Map<string, Map<string, boolean>>;
	readonly shorthandConfiguration: ShorthandConfiguration;
}

export interface NameReplacements {
	samples?: ReadonlyArray<string>;
	total: number;
}

export type NamedIdentifier = ESTree.BindingIdentifier | ESTree.IdentifierName | ESTree.IdentifierReference;
export type BroadIdentifier =
	| ESTree.LabelIdentifier
	| ESTree.TSIndexSignatureName
	| ESTree.TSThisParameter
	| NamedIdentifier;

export interface VariableLike {
	readonly defs: ReadonlyArray<Definition>;
	readonly identifiers: ReadonlyArray<BroadIdentifier>;
	readonly name: string;
	readonly references: ReadonlyArray<Reference>;
	readonly scope: Scope;
}

export type IsSafe = (name: string, scopes: ReadonlyArray<Scope>) => boolean;

export interface NodeRange {
	readonly range: [number, number];
}
