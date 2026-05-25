#!/usr/bin/env bun

import { readFile, rm, stat } from "node:fs/promises";
import { basename, extname, resolve } from "node:path";
import { performance } from "node:perf_hooks";
import { argv, cwd, exit } from "node:process";
import { Command } from "@cliffy/command";
import { regex } from "arktype";
import { consola } from "consola";
import prettyBytes from "pretty-bytes";
import prettyMilliseconds from "pretty-ms";
import { build } from "tsdown";
import { bold, cyan, gray, green, magenta, red, yellow } from "yoctocolors";
import { $ } from "zx";

const scriptPath = import.meta.filename;
const scriptName = basename(scriptPath, extname(scriptPath));
const entryPoints: ReadonlyArray<string> = ["./plugins/oxc/small-rules/index.ts"];
const externalPackages: ReadonlyArray<string> = [
	"@oxlint/plugins",
	"type-fest",
	"oxc-parser",
	"oxc-resolver",
	"arktype",
];
const javaScriptOutputPath = resolve("plugins/oxc/small-rules.js");
const sourceMapOutputPath = resolve("plugins/oxc/small-rules.js.map");
const pluginTypeScriptConfigurationPath = "./tsconfig.plugins.json";

interface BuildOptions {
	readonly clean: boolean;
	readonly minify: boolean;
	readonly sourcemap: boolean;
	readonly verbose: boolean;
}

interface OutputFile {
	readonly path: string;
	readonly size: number;
}

interface BuildResult {
	readonly duration: number;
	readonly files: ReadonlyArray<OutputFile>;
	readonly success: boolean;
}

function getBooleanString(boolean: boolean): string {
	return boolean ? green("yes") : gray("no");
}

async function removeAsync(filePath: string, verbose: boolean): Promise<void> {
	try {
		await stat(filePath);
	} catch {
		return;
	}

	if (verbose) consola.info(`Removing ${cyan(filePath)}...`);
	await rm(filePath);
}

async function cleanOutputFilesAsync(verbose: boolean): Promise<void> {
	await Promise.all([removeAsync(javaScriptOutputPath, verbose), removeAsync(sourceMapOutputPath, verbose)]);
}

async function getOutputFilesAsync(sourcemap: boolean): Promise<ReadonlyArray<OutputFile>> {
	const javaScriptStatistics = await stat(javaScriptOutputPath);
	const files = [
		{
			path: javaScriptOutputPath.replace(`${cwd()}/`, ""),
			size: javaScriptStatistics.size,
		},
	];

	if (sourcemap) {
		const sourceMapStatistics = await stat(sourceMapOutputPath);
		files[1] = {
			path: sourceMapOutputPath.replace(`${cwd()}/`, ""),
			size: sourceMapStatistics.size,
		};
	}

	return files.toSorted((left, right) => left.path.localeCompare(right.path));
}

async function runBuildAsync(buildOptions: BuildOptions): Promise<BuildResult> {
	const startTime = performance.now();

	try {
		if (buildOptions.clean) {
			if (buildOptions.verbose) consola.start("Cleaning output files...");
			await cleanOutputFilesAsync(buildOptions.verbose);
			if (buildOptions.verbose) consola.success("Cleaned output files");
		}

		if (buildOptions.verbose) {
			consola.start("Building with tsdown...");
			consola.info(`  Entry points: ${cyan(entryPoints.join(", "))}`);
			consola.info(`  Minify: ${getBooleanString(buildOptions.minify)}`);
			consola.info(`  Sourcemap: ${getBooleanString(buildOptions.sourcemap)}`);
			consola.info(`  TypeScript config: ${cyan(pluginTypeScriptConfigurationPath)}`);
			consola.info(`  Output file: ${cyan(javaScriptOutputPath)}`);
		}

		const [entryPoint] = entryPoints;
		if (entryPoint === undefined) {
			const error = new Error("No entry point defined");
			Error.captureStackTrace(error, runBuildAsync);
			throw error;
		}

		await build({
			clean: false,
			entry: { "small-rules": entryPoint },
			external: [...externalPackages],
			fixedExtension: false,
			format: ["esm"],
			logLevel: "silent",
			minify: buildOptions.minify,
			outDir: "plugins/oxc",
			platform: "node",
			sourcemap: buildOptions.sourcemap,
			tsconfig: pluginTypeScriptConfigurationPath,
		});

		const files = await getOutputFilesAsync(buildOptions.sourcemap);
		const duration = performance.now() - startTime;

		return { duration, files, success: true };
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		consola.error(`${red("error:")} ${message}`);

		return {
			duration: performance.now() - startTime,
			files: [],
			success: false,
		};
	}
}

function printBuildSummary(buildResult: BuildResult, verbose: boolean): void {
	if (!buildResult.success) {
		consola.fail(red(`Build failed in ${prettyMilliseconds(buildResult.duration)}`));
		return;
	}

	const { files } = buildResult;

	let javaScriptFiles = 0;
	let sourceMapFiles = 0;
	let totalSize = 0;

	for (const { path, size } of files) {
		totalSize += size;
		if (path.endsWith(".js")) javaScriptFiles += 1;
		else if (path.endsWith(".js.map")) sourceMapFiles += 1;
	}

	consola.log("");
	consola.success(green(bold("Build completed successfully!")));
	consola.log("");

	if (verbose) {
		consola.info(bold("Output files:"));
		for (const { path, size } of files) {
			const color = path.endsWith(".js.map") ? gray : cyan;
			const bytes = gray(`(${prettyBytes(size)})`);
			consola.log(`  ${color(path)} ${bytes}`);
		}
		consola.log("");
	}

	consola.info(bold("Summary:"));
	// oxlint-disable-next-line no-script-url
	consola.log(`  ${cyan("JavaScript:")} ${javaScriptFiles} files`);
	if (sourceMapFiles > 0) consola.log(`  ${gray("Sourcemaps:")} ${sourceMapFiles} files`);
	consola.log(`  ${magenta("Total size:")} ${prettyBytes(totalSize)}`);
	consola.log(`  ${green("Duration:")} ${prettyMilliseconds(buildResult.duration)}`);
}

const MATCH_LINE = regex(
	// oxlint-disable-next-line unicorn/prefer-string-raw
	"^(?<filePath>.+?)\\((?<lineNumberString>\\d+),(?<columnNumberString>\\d+)\\): (?<level>error|warning) (?<code>TS\\d+): (?<message>.+)$",
	"u",
);
const CARRIAGE_RETURN = /\r$/u;

async function getSourceLinesAsync(
	fileCache: Map<string, ReadonlyArray<string>>,
	filePath: string,
): Promise<ReadonlyArray<string> | undefined> {
	const cached = fileCache.get(filePath);
	if (cached) return cached;

	try {
		const fileContent = await readFile(filePath, "utf8");
		const sourceLines = fileContent.split("\n");
		fileCache.set(filePath, sourceLines);
		return sourceLines;
	} catch {
		return undefined;
	}
}

const TAB_REGEXP = /\t/gv;

async function printSourceContextAsync(
	fileCache: Map<string, ReadonlyArray<string>>,
	filePath: string,
	lineNumber: number,
	lineNumberString: string,
	columnNumber: number,
): Promise<void> {
	const sourceLines = await getSourceLinesAsync(fileCache, filePath);
	if (!sourceLines) return;

	const sourceLine = sourceLines[lineNumber - 1]?.replace(CARRIAGE_RETURN, "");
	if (sourceLine === undefined) return;

	const displayLine = sourceLine.replaceAll("	", "    ");
	const tabCount = (sourceLine.slice(0, columnNumber - 1).match(TAB_REGEXP) ?? []).length;
	const displayColumn = columnNumber - 1 + tabCount * 3;

	consola.log(`${gray(lineNumberString)} | ${displayLine}`);
	const padding = " ".repeat(lineNumberString.length + 3 + displayColumn);
	consola.log(`${padding}${red("^")}`);
}

async function validateTypesAsync(verbose: boolean): Promise<void> {
	if (verbose) consola.start("Validating types...");
	const startTime = performance.now();

	const shellOutput = await $`aube run type-check -- --project ./tsconfig.plugins.json`.quiet().nothrow();
	const duration = performance.now() - startTime;

	if (shellOutput.exitCode === 0) {
		if (verbose) consola.success(`Types validated in ${prettyMilliseconds(duration)}`);
		return;
	}

	consola.fail(red(`Type validation failed in ${prettyMilliseconds(duration)}`));
	consola.log("");

	const stdout = shellOutput.stdout.trim();
	if (stdout) {
		const lines = stdout.split("\n");
		const fileCache = new Map<string, ReadonlyArray<string>>();

		for (const line of lines) {
			const match = MATCH_LINE.exec(line);
			if (match) {
				const { code, columnNumberString, filePath, level, lineNumberString, message } = match.groups;

				const relativePath = filePath.replace(`${cwd()}/`, "");
				const lineNumber = Number.parseInt(lineNumberString, 10);
				const columnNumber = Number.parseInt(columnNumberString, 10);

				consola.log(`${cyan(relativePath)}:${yellow(lineNumberString)}:${yellow(columnNumberString)}`);

				try {
					// oxlint-disable-next-line no-await-in-loop
					await printSourceContextAsync(fileCache, filePath, lineNumber, lineNumberString, columnNumber);
				} catch {
					// Keep the original type-check output even if context extraction fails.
				}

				const levelText = level === "error" ? red("error:") : yellow("warning:");
				const codeText = gray(`(${code})`);
				consola.log(`${levelText} ${message} ${codeText}\n`);
			} else if (line.trim() !== "") consola.log(gray(line));
		}
	}

	const stderr = shellOutput.stderr.trim();
	if (stderr) consola.error(`\n${red(stderr)}`);

	exit(1);
}

const command = new Command()
	.name(scriptName)
	.version("2.0.0")
	.description("Build the Oxlint plugin for distribution.")
	.option("--no-clean", "Skip cleaning existing outputs before build", { default: true })
	.option("-v, --verbose", "Show detailed build output", { default: false })
	.option("-m, --minify", "Aggressively minify identifiers and syntax", { default: false })
	.option("--sourcemap", "Generate a sourcemap next to the built plugin", { default: false })
	.action(async ({ clean, minify, sourcemap, verbose }) => {
		if (verbose) {
			consola.info(bold("Build configuration:"));
			consola.log(`  Clean: ${getBooleanString(clean)}`);
			consola.log(`  Minify: ${getBooleanString(minify)}`);
			consola.log(`  Sourcemap: ${getBooleanString(sourcemap)}`);
			consola.log("");
		}

		await validateTypesAsync(verbose);

		const buildResult = await runBuildAsync({ clean, minify, sourcemap, verbose });
		printBuildSummary(buildResult, verbose);

		if (!buildResult.success) exit(1);
	});

const scriptIndex = argv.findIndex((argument) => resolve(argument) === import.meta.filename);
await command.parse(argv.slice(scriptIndex + 1));
