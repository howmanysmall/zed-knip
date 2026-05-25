import { isRecord } from "@oxlint-utilities/type-utilities";
import { defineRule } from "oxlint-plugin-utilities";

import type { ESTree, Visitor } from "oxlint-plugin-utilities";

type SwitchCaseBraceMetric = "lines" | "statements";

function normalizeMetric(rawOptions: unknown): SwitchCaseBraceMetric {
	if (!isRecord(rawOptions)) return "lines";
	if (rawOptions.metric === "statements") return "statements";
	return "lines";
}

function shouldReportSwitchCaseBraces(node: ESTree.SwitchCase, metric: SwitchCaseBraceMetric): boolean {
	const consequentCount = node.consequent.length;
	if (consequentCount === 0) return false;

	const [firstStatement] = node.consequent;
	if (firstStatement === undefined || (consequentCount === 1 && firstStatement.type === "BlockStatement")) {
		return false;
	}

	if (metric === "statements") return consequentCount > 1;

	const lastStatement = node.consequent[consequentCount - 1];
	if (lastStatement === undefined) return false;

	return firstStatement.loc.start.line !== lastStatement.loc.end.line;
}

const requireSwitchCaseBraces = defineRule({
	create(context): Visitor {
		const metric = normalizeMetric(context.options[0]);

		return {
			SwitchCase(node): void {
				if (!shouldReportSwitchCaseBraces(node, metric)) return;

				const [firstStatement] = node.consequent;
				const lastStatement = node.consequent.at(-1);
				if (firstStatement === undefined || lastStatement === undefined) return;

				context.report({
					fix(fixer) {
						return [
							fixer.insertTextBefore(firstStatement, "{\n"),
							fixer.insertTextAfter(lastStatement, "\n}"),
						];
					},
					messageId: "wrapCaseBody",
					node: firstStatement,
				});
			},
		} satisfies Visitor;
	},
	meta: {
		docs: {
			description: "Require braces around switch case bodies that span multiple lines.",
		},
		fixable: "code",
		messages: {
			wrapCaseBody: "Wrap this switch case body in braces.",
		},
		schema: [
			{
				additionalProperties: false,
				properties: {
					metric: {
						default: "lines",
						enum: ["lines", "statements"],
						type: "string",
					},
				},
				type: "object",
			},
		],
		type: "problem",
	},
});

export default requireSwitchCaseBraces;
