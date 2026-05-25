import { MetaSelectors, Modifiers, PredefinedFormats, Selectors, UnderscoreOptions } from "./enums";
import { getEnumNames } from "./get-enum-names";

export const SCHEMA = [
	{
		additionalProperties: false,
		description: "Multiple selectors in one config",
		properties: {
			custom: {
				additionalProperties: false,
				properties: {
					match: { type: "boolean" },
					regex: { type: "string" },
				},
				required: ["match", "regex"],
				type: "object",
			},
			filter: {
				oneOf: [
					{ minLength: 1, type: "string" },
					{
						additionalProperties: false,
						properties: {
							match: { type: "boolean" },
							regex: { type: "string" },
						},
						required: ["match", "regex"],
						type: "object",
					},
				],
			},
			format: {
				oneOf: [
					{
						additionalItems: false,
						items: { enum: getEnumNames(PredefinedFormats), type: "string" },
						type: "array",
					},
					{ type: "null" },
				],
			},
			leadingUnderscore: { enum: getEnumNames(UnderscoreOptions), type: "string" },
			modifiers: {
				additionalItems: false,
				items: { enum: getEnumNames(Modifiers), type: "string" },
				type: "array",
			},
			prefix: {
				additionalItems: false,
				items: { minLength: 1, type: "string" },
				type: "array",
			},
			selector: {
				oneOf: [
					{ enum: [...getEnumNames(MetaSelectors), ...getEnumNames(Selectors)], type: "string" },
					{
						additionalItems: false,
						items: { enum: [...getEnumNames(MetaSelectors), ...getEnumNames(Selectors)], type: "string" },
						type: "array",
					},
				],
			},
			suffix: {
				additionalItems: false,
				items: { minLength: 1, type: "string" },
				type: "array",
			},
			trailingUnderscore: { enum: getEnumNames(UnderscoreOptions), type: "string" },
		},
		required: ["selector", "format"],
		type: "object",
	},
] as const;
