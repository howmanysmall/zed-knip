import { extname } from "node:path";
import { hasCodeLines } from "@oxlint-utilities/recognizers/code-recognizer";
import { createJavaScriptDetectors } from "@oxlint-utilities/recognizers/javascript-footprint";
import { isRecord } from "@oxlint-utilities/type-utilities";
import { parseSync } from "oxc-parser";
import { defineRule } from "oxlint-plugin-utilities";

import type { Comment, ESTree, Fix, SourceCode, Visitor } from "oxlint-plugin-utilities";

const EXCLUDED_STATEMENTS = new Set(["BreakStatement", "ContinueStatement", "LabeledStatement"]);
function isExcludedStatement(
	statement: ESTree.Statement,
): statement is ESTree.BreakStatement | ESTree.ContinueStatement | ESTree.LabeledStatement {
	return EXCLUDED_STATEMENTS.has(statement.type);
}

interface CommentWithLocation extends Comment {
	readonly loc: NonNullable<Comment["loc"]>;
	readonly range: [number, number];
}

interface CommentGroup {
	readonly comments: ReadonlyArray<CommentWithLocation>;
	readonly value: string;
}

const detectors = createJavaScriptDetectors();

function areAdjacentLineComments(
	previous: CommentWithLocation,
	next: CommentWithLocation,
	sourceCode: SourceCode,
): boolean {
	const previousLine = previous.loc.start.line;
	const nextLine = next.loc.start.line;
	if (previousLine + 1 !== nextLine) return false;

	const tokenAfterPrevious = sourceCode.getTokenAfter(previous);
	if (!tokenAfterPrevious) return true;
	return tokenAfterPrevious.loc.start.line > nextLine;
}

function groupComments(comments: ReadonlyArray<Comment>, sourceCode: SourceCode): Array<CommentGroup> {
	const groups = new Array<CommentGroup>();
	let groupsSize = 0;
	let currentLineComments = new Array<CommentWithLocation>();
	let size = 0;

	for (const comment of comments) {
		if (comment.type === "Block") {
			if (size > 0) {
				groups[groupsSize++] = {
					comments: currentLineComments,
					value: currentLineComments.map(({ value }) => value).join("\n"),
				};
				currentLineComments = [];
				size = 0;
			}
			groups[groupsSize++] = {
				comments: [comment],
				value: comment.value,
			};
		} else if (size === 0) currentLineComments[size++] = comment;
		else {
			const lastComment = currentLineComments.at(-1);
			if (lastComment && areAdjacentLineComments(lastComment, comment, sourceCode)) {
				currentLineComments[size++] = comment;
			} else {
				groups[groupsSize++] = {
					comments: currentLineComments,
					value: currentLineComments.map(({ value }) => value).join("\n"),
				};
				currentLineComments = [comment];
				size = 1;
			}
		}
	}

	if (size > 0) {
		groups[groupsSize] = {
			comments: currentLineComments,
			value: currentLineComments.map(({ value }) => value).join("\n"),
		};
	}

	return groups;
}

const OPENING_BRACE = /\{/gv;
const CLOSING_BRACE = /\}/gv;
function injectMissingBraces(value: string): string {
	const openCount = (value.match(OPENING_BRACE) ?? []).length;
	const closeCount = (value.match(CLOSING_BRACE) ?? []).length;
	const diff = openCount - closeCount;

	if (diff > 0) return value + "}".repeat(diff);
	if (diff < 0) return "{".repeat(-diff) + value;
	return value;
}

function couldBeJsCode(input: string): boolean {
	return hasCodeLines(detectors, input.split("\n"));
}

function isReturnOrThrowExclusion(statement: ESTree.Statement): boolean {
	if (statement.type !== "ReturnStatement" && statement.type !== "ThrowStatement") return false;
	return statement.argument?.type === "Identifier";
}

function isUnaryPlusMinus(expression: ESTree.Expression): boolean {
	return expression.type === "UnaryExpression" && (expression.operator === "-" || expression.operator === "+");
}

function isExcludedLiteral(expression: { type: string; value?: unknown }): boolean {
	return (
		expression.type === "Literal" && (typeof expression.value === "string" || typeof expression.value === "number")
	);
}

function isParsedStatement(value: unknown): value is ESTree.Statement {
	return isRecord(value) && typeof value.type === "string";
}

function toParsedStatements(body: ReadonlyArray<unknown>): ReadonlyArray<ESTree.Statement> {
	const result = new Array<ESTree.Statement>();
	let size = 0;
	for (const item of body) if (isParsedStatement(item)) result[size++] = item;
	return result;
}

function isExpressionExclusion(statement: ESTree.Statement, codeText: string): boolean {
	if (statement.type !== "ExpressionStatement") return false;

	const { expression } = statement;
	return (
		expression.type === "Identifier" ||
		expression.type === "SequenceExpression" ||
		isUnaryPlusMinus(expression) ||
		isExcludedLiteral(expression) ||
		!codeText.trimEnd().endsWith(";")
	);
}

function isExclusion(statements: ReadonlyArray<ESTree.Statement>, codeText: string): boolean {
	if (statements.length !== 1) return false;

	const statement = statements.at(0);
	if (!statement) return false;

	return (
		isExcludedStatement(statement) ||
		isReturnOrThrowExclusion(statement) ||
		isExpressionExclusion(statement, codeText)
	);
}

const ALLOWED_PARSE_ERROR_PATTERNS = [/A 'return' statement can only be used within a function body/v] as const;
type Errors = ReadonlyArray<{ readonly message: string }>;
function hasOnlyAllowedErrors(errors: Errors): boolean {
	for (const error of errors) {
		let hasMatch = false;
		for (const pattern of ALLOWED_PARSE_ERROR_PATTERNS) {
			if (pattern.test(error.message)) {
				hasMatch = true;
				break;
			}
		}
		if (!hasMatch) return false;
	}
	return true;
}

interface Body {
	readonly body: ReadonlyArray<unknown>;
}

interface ParseResult {
	readonly errors: Errors;
	readonly program: Body;
}

function isValidParseResult(result: ParseResult): boolean {
	const hasValidErrors = result.errors.length === 0 || hasOnlyAllowedErrors(result.errors);
	return hasValidErrors && result.program.body.length > 0;
}

function tryParse(value: string, filename: string): ParseResult | undefined {
	const extension = extname(filename);
	const parseFilename = `file${extension || ".js"}`;
	const result = parseSync(parseFilename, value);

	if (isValidParseResult(result)) return result;

	if (extension !== ".tsx" && extension !== ".jsx") {
		const jsxResult = parseSync("file.tsx", value);
		if (isValidParseResult(jsxResult)) return jsxResult;
	}

	return undefined;
}

function containsCode(value: string, filename: string): boolean {
	if (!couldBeJsCode(value)) return false;

	const result = tryParse(value, filename);
	if (!result) return false;

	const statements = toParsedStatements(result.program.body);
	return !isExclusion(statements, value);
}

const noCommentedCode = defineRule({
	create(context): Visitor {
		return {
			"Program:exit": function (): void {
				const allComments = context.sourceCode.getAllComments();
				const groups = groupComments(allComments, context.sourceCode);

				for (const group of groups) {
					const trimmedValue = group.value.trim();
					if (trimmedValue === "}") continue;

					const balanced = injectMissingBraces(trimmedValue);
					if (!containsCode(balanced, context.filename)) continue;

					const firstComment = group.comments.at(0);
					const lastComment = group.comments.at(-1);
					if (!firstComment || !lastComment) continue;

					context.report({
						loc: {
							end: lastComment.loc.end,
							start: firstComment.loc.start,
						},
						messageId: "commentedCode",
						suggest: [
							{
								desc: "Remove this commented out code",
								fix(fixer): Fix {
									return fixer.removeRange([firstComment.range[0], lastComment.range[1]]);
								},
							},
						],
					});
				}
			},
		} satisfies Visitor;
	},
	meta: {
		docs: {
			description: "Disallow commented-out code",
			recommended: false,
		},
		hasSuggestions: true,
		messages: {
			commentedCode:
				"Commented-out code creates confusion about intent and clutters the codebase. Version control preserves history, making dead code comments unnecessary. Delete the commented code entirely. If needed later, retrieve it from git history.",
		},
		schema: [],
		type: "suggestion",
	},
});

export default noCommentedCode;
