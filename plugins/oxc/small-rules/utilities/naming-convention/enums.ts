import { type } from "arktype";

export const PredefinedFormats = {
	camelCase: "camelCase",
	PascalCase: "PascalCase",
	snake_case: "snake_case",
	strictCamelCase: "strictCamelCase",
	StrictPascalCase: "StrictPascalCase",
	UPPER_CASE: "UPPER_CASE",
} as const;
export const isPredefinedFormatsString = type.enumerated(...Object.values(PredefinedFormats));
export type PredefinedFormatsString = typeof isPredefinedFormatsString.infer;
export type PredefinedFormats = (typeof PredefinedFormats)[PredefinedFormatsString];

export const UnderscoreOptions = {
	allow: "allow",
	allowDouble: "allowDouble",
	allowSingleOrDouble: "allowSingleOrDouble",
	forbid: "forbid",
	require: "require",
	requireDouble: "requireDouble",
} as const;
export const isUnderscoreOptionsString = type.enumerated(...Object.values(UnderscoreOptions));
export type UnderscoreOptionsString = typeof isUnderscoreOptionsString.infer;
export type UnderscoreOptions = (typeof UnderscoreOptions)[UnderscoreOptionsString];

export const Selectors = {
	autoAccessor: "autoAccessor",
	class: "class",
	classicAccessor: "classicAccessor",
	classMethod: "classMethod",
	classProperty: "classProperty",
	enum: "enum",
	enumMember: "enumMember",
	function: "function",
	import: "import",
	interface: "interface",
	objectLiteralMethod: "objectLiteralMethod",
	objectLiteralProperty: "objectLiteralProperty",
	parameter: "parameter",
	parameterProperty: "parameterProperty",
	typeAlias: "typeAlias",
	typeMethod: "typeMethod",
	typeParameter: "typeParameter",
	typeProperty: "typeProperty",
	variable: "variable",
} as const;
export const isSelectorsString = type.enumerated(...Object.values(Selectors));
export type SelectorsString = typeof isSelectorsString.infer;
export type Selectors = (typeof Selectors)[SelectorsString];

export const MetaSelectors = {
	accessor: "accessor",
	default: "default",
	memberLike: "memberLike",
	method: "method",
	property: "property",
	typeLike: "typeLike",
	variableLike: "variableLike",
} as const;
export const isMetaSelectorsString = type.enumerated(...Object.values(MetaSelectors));
export type MetaSelectorsString = typeof isMetaSelectorsString.infer;

export const isIndividualAndMetaSelectorsString = isMetaSelectorsString.or(isSelectorsString);
export type IndividualAndMetaSelectorsString = typeof isIndividualAndMetaSelectorsString.infer;

export const Modifiers = {
	"#private": "#private",
	abstract: "abstract",
	async: "async",
	const: "const",
	default: "default",
	destructured: "destructured",
	exported: "exported",
	global: "global",
	namespace: "namespace",
	override: "override",
	private: "private",
	protected: "protected",
	public: "public",
	readonly: "readonly",
	requiresQuotes: "requiresQuotes",
	static: "static",
	unused: "unused",
} as const;
export const isModifiersString = type.enumerated(...Object.values(Modifiers));
export type ModifiersString = typeof isModifiersString.infer;
export type Modifiers = (typeof Modifiers)[ModifiersString];

export const ModifierWeights: Record<ModifiersString, number> = {
	"#private": 64,
	abstract: 128,
	async: 2 ** 14,
	const: 1,
	default: 2 ** 15,
	destructured: 256,
	exported: 2 ** 10,
	global: 512,
	namespace: 2 ** 16,
	override: 2 ** 13,
	private: 32,
	protected: 16,
	public: 8,
	readonly: 2,
	requiresQuotes: 2 ** 12,
	static: 4,
	unused: 2 ** 11,
};
