import arrayTypeGeneric from "@oxlint-rules/array-type-generic";
import banTypes from "@oxlint-rules/ban-types";
import noArrayConstructorElements from "@oxlint-rules/no-array-constructor-elements";
import noArraySizeAssignment from "@oxlint-rules/no-array-size-assignment";
import noAsyncConstructor from "@oxlint-rules/no-async-constructor";
import noCommentedCode from "@oxlint-rules/no-commented-code";
import noConstantConditionWithBreak from "@oxlint-rules/no-constant-condition-with-break";
import noInstanceMethodsWithoutThis from "@oxlint-rules/no-instance-methods-without-this";
import noUnusedImports from "@oxlint-rules/no-unused-imports";
import preferClassProperties from "@oxlint-rules/prefer-class-properties";
import preferEarlyReturn from "@oxlint-rules/prefer-early-return";
import preferExpectAssertions from "@oxlint-rules/prefer-expect-assertions";
import preferModuleScopeConstants from "@oxlint-rules/prefer-module-scope-constants";
import preferPascalCaseEnums from "@oxlint-rules/prefer-pascal-case-enums";
import preferSingularEnums from "@oxlint-rules/prefer-singular-enums";
import preventAbbreviations from "@oxlint-rules/prevent-abbreviations";
import requireAsyncSuffix from "@oxlint-rules/require-async-suffix";
import requireModuleLevelInstantiation from "@oxlint-rules/require-module-level-instantiation";
import requireSwitchCaseBraces from "@oxlint-rules/require-switch-case-braces";
import requireUnicodeRegex from "@oxlint-rules/require-unicode-regex";
import strictComponentBoundaries from "@oxlint-rules/strict-component-boundaries";
import { definePlugin } from "oxlint-plugin-utilities";

import noUselessConstants from "./rules/no-useless-constants";

const smallRules = definePlugin({
	meta: { name: "small-rules" },
	rules: {
		"array-type-generic": arrayTypeGeneric,
		"ban-types": banTypes,
		"no-array-constructor-elements": noArrayConstructorElements,
		"no-array-size-assignment": noArraySizeAssignment,
		"no-async-constructor": noAsyncConstructor,
		"no-commented-code": noCommentedCode,
		"no-constant-condition-with-break": noConstantConditionWithBreak,
		"no-instance-methods-without-this": noInstanceMethodsWithoutThis,
		"no-unused-imports": noUnusedImports,
		"no-useless-constants": noUselessConstants,
		"prefer-class-properties": preferClassProperties,
		"prefer-early-return": preferEarlyReturn,
		"prefer-expect-assertions": preferExpectAssertions,
		"prefer-module-scope-constants": preferModuleScopeConstants,
		"prefer-pascal-case-enums": preferPascalCaseEnums,
		"prefer-singular-enums": preferSingularEnums,
		"prevent-abbreviations": preventAbbreviations,
		"require-async-suffix": requireAsyncSuffix,
		"require-module-level-instantiation": requireModuleLevelInstantiation,
		"require-switch-case-braces": requireSwitchCaseBraces,
		"require-unicode-regex": requireUnicodeRegex,
		"strict-component-boundaries": strictComponentBoundaries,
	},
});

export default smallRules;
