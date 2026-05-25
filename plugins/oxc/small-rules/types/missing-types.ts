import type { ESTree } from "oxlint-plugin-utilities";

export type BindingName = ESTree.BindingIdentifier | ESTree.BindingPattern;
export type CallbackFunction = ESTree.ArrowFunctionExpression | ESTree.Function;
