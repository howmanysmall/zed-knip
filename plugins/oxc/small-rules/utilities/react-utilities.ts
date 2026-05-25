import { isRecord } from "@oxlint-utilities/type-utilities";

import type { ESTree } from "oxlint-plugin-utilities";

export type Environment = "roblox-ts" | "standard";

export const ENVIRONMENT_SCHEMA = {
	enum: ["roblox-ts", "standard"],
	type: "string",
};

export function isEnvironment(value: unknown): value is Environment {
	return value === "roblox-ts" || value === "standard";
}

const STANDARD_REACT_SOURCES = new Set<string>(["react", "react-dom"]);
const ROBLOX_TS_REACT_SOURCES = new Set<string>(["@rbxts/react", "@rbxts/roact"]);

export function getReactSources(environment: Environment): ReadonlySet<string> {
	if (environment === "standard") return STANDARD_REACT_SOURCES;
	return ROBLOX_TS_REACT_SOURCES;
}

export function getReactSourcesFromOptions(value: unknown): ReadonlySet<string> {
	return getReactSources(getEnvironment(value));
}

export function isReactImport(node: ESTree.ImportDeclaration, reactSources: ReadonlySet<string>): boolean {
	return reactSources.has(node.source.value);
}

export function getEnvironment(value: unknown): Environment {
	if (!isRecord(value) || value.environment !== "standard") return "roblox-ts";
	return "standard";
}
