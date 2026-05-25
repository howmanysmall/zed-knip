import { existsSync, readdirSync, readFileSync } from "node:fs";
import { basename, dirname, extname, join, relative } from "node:path";

interface LocalComponentDefinition {
	readonly componentName: string;
	readonly fileNames: ReadonlyArray<string>;
	readonly markers?: ReadonlyArray<string>;
}

interface LocalComponentInspection {
	readonly importStyle: "default" | "named" | undefined;
	readonly matches: boolean;
}

type LocalComponentDiscovery =
	| { readonly found: false }
	| {
			readonly found: true;
			readonly importSource: string;
			readonly importStyle: "default" | "named" | undefined;
			readonly path: string;
	  };

const COMPONENT_EXTENSIONS = new Set([".js", ".jsx", ".ts", ".tsx"]);
const IGNORED_DIRECTORIES = new Set([
	".git",
	".next",
	".turbo",
	"__fixtures__",
	"__mocks__",
	"__tests__",
	"coverage",
	"dist",
	"documentation",
	"example",
	"examples",
	"fixture",
	"fixtures",
	"node_modules",
	"opensrc",
	"stories",
	"story",
	"test",
	"tests",
]);
const DEFAULT_EXPORT_PATTERN = /\bexport\s+default\b/v;
const INDEX_SUFFIX_PATTERN = /\/index$/v;
const SOURCE_EXTENSION_PATTERN = /\.(?:[cm]?jsx?|tsx?)$/v;

const fileIndexCache = new Map<string, ReadonlyMap<string, ReadonlyArray<string>>>();
const fileTextCache = new Map<string, string>();
const projectRootCache = new Map<string, string | undefined>();

const REGEXP_REGEXP = /[.*+?^${}()|[\]\\]/gu;
const AND = String.raw`\$&`;
function escapeRegExp(value: string): string {
	return value.replaceAll(REGEXP_REGEXP, AND);
}

function getFileText(filePath: string): string {
	const cached = fileTextCache.get(filePath);
	if (cached !== undefined) return cached;

	const text = readFileSync(filePath, "utf8");
	fileTextCache.set(filePath, text);
	return text;
}

function getImportStyle(text: string, componentName: string): "default" | "named" | undefined {
	const escapedName = escapeRegExp(componentName);
	const hasNamedExport =
		new RegExp(`\\bexport\\s+(?:const|function|class)\\s+${escapedName}\\b`, "v").test(text) ||
		new RegExp(`\\bexport\\s*\\{[^}]*\\b${escapedName}\\b[^}]*\\}`, "v").test(text);
	if (hasNamedExport) return "named";

	return DEFAULT_EXPORT_PATTERN.test(text) && new RegExp(`\\b${escapedName}\\b`, "v").test(text)
		? "default"
		: undefined;
}

function hasExpectedMarkers(text: string, markers: ReadonlyArray<string>): boolean {
	for (const marker of markers) {
		if (!new RegExp(`\\b${escapeRegExp(marker)}\\b`, "v").test(text)) return false;
	}

	return true;
}

function getProjectRootFromDirectory(startDirectory: string): string | undefined {
	const cached = projectRootCache.get(startDirectory);
	if (cached !== undefined) return cached;

	let currentDirectory = startDirectory;
	while (true) {
		if (existsSync(join(currentDirectory, "package.json")) || existsSync(join(currentDirectory, "tsconfig.json"))) {
			projectRootCache.set(startDirectory, currentDirectory);
			return currentDirectory;
		}

		const parentDirectory = dirname(currentDirectory);
		if (parentDirectory === currentDirectory) {
			projectRootCache.set(startDirectory, undefined);
			return undefined;
		}

		currentDirectory = parentDirectory;
	}
}

function indexProjectFiles(rootDirectory: string): ReadonlyMap<string, ReadonlyArray<string>> {
	const cached = fileIndexCache.get(rootDirectory);
	if (cached !== undefined) return cached;

	const index = new Map<string, Array<string>>();

	function visit(directory: string): void {
		for (const entry of readdirSync(directory, { withFileTypes: true })) {
			const entryName = entry.name.toLowerCase();
			if ((entry.name.startsWith(".") && entry.name !== ".storybook") || IGNORED_DIRECTORIES.has(entryName)) {
				continue;
			}

			const fullPath = join(directory, entry.name);
			if (entry.isDirectory()) {
				visit(fullPath);
				continue;
			}

			const extension = extname(entry.name);
			if (!COMPONENT_EXTENSIONS.has(extension) || entry.name.endsWith(".d.ts")) continue;

			const baseName = basename(entry.name, extension).toLowerCase();
			const existing = index.get(baseName);
			if (existing === undefined) index.set(baseName, [fullPath]);
			else existing.push(fullPath);
		}
	}

	visit(rootDirectory);

	const readonlyIndex = new Map<string, ReadonlyArray<string>>();
	for (const [key, value] of index) readonlyIndex.set(key, value);

	fileIndexCache.set(rootDirectory, readonlyIndex);
	return readonlyIndex;
}

function toImportSource(sourceFile: string, targetFile: string): string {
	let importSource = relative(dirname(sourceFile), targetFile).replaceAll("\\", "/");
	importSource = importSource.replace(SOURCE_EXTENSION_PATTERN, "");
	importSource = importSource.replace(INDEX_SUFFIX_PATTERN, "");

	if (!importSource.startsWith(".")) importSource = `./${importSource}`;
	return importSource;
}

function isIgnoredComponentPath(filePath: string): boolean {
	const projectRoot = getProjectRootFromDirectory(dirname(filePath));
	const normalizedPath = (projectRoot === undefined ? filePath : relative(projectRoot, filePath)).replaceAll(
		"\\",
		"/",
	);
	for (const segment of normalizedPath.split("/")) {
		if (IGNORED_DIRECTORIES.has(segment.toLowerCase())) return true;
	}

	return false;
}

export function inspectLocalComponentFile(
	filePath: string,
	definition: LocalComponentDefinition,
): LocalComponentInspection {
	if (isIgnoredComponentPath(filePath)) return { importStyle: undefined, matches: false };

	const extension = extname(filePath);
	if (!COMPONENT_EXTENSIONS.has(extension) || filePath.endsWith(".d.ts")) {
		return { importStyle: undefined, matches: false };
	}

	const baseName = basename(filePath, extension).toLowerCase();
	const fileNames = definition.fileNames.map((fileName) => fileName.toLowerCase());
	if (!fileNames.includes(baseName)) return { importStyle: undefined, matches: false };

	const text = getFileText(filePath);
	if (!new RegExp(`\\b${escapeRegExp(definition.componentName)}\\b`, "v").test(text)) {
		return { importStyle: undefined, matches: false };
	}

	if (definition.markers !== undefined && !hasExpectedMarkers(text, definition.markers)) {
		return { importStyle: undefined, matches: false };
	}

	const importStyle = getImportStyle(text, definition.componentName);
	if (importStyle === undefined) return { importStyle: undefined, matches: false };

	return {
		importStyle,
		matches: true,
	};
}

export function discoverLocalComponent(
	sourceFile: string,
	definition: LocalComponentDefinition,
): LocalComponentDiscovery {
	const projectRoot = getProjectRootFromDirectory(dirname(sourceFile));
	if (projectRoot === undefined) return { found: false };

	const projectFiles = indexProjectFiles(projectRoot);
	const candidatePaths = new Set<string>();
	for (const fileName of definition.fileNames) {
		for (const candidatePath of projectFiles.get(fileName.toLowerCase()) ?? []) candidatePaths.add(candidatePath);
	}

	const matches = [...candidatePaths].filter(
		(candidatePath) => inspectLocalComponentFile(candidatePath, definition).matches,
	);
	if (matches.length !== 1) return { found: false };

	const [path] = matches;
	if (path === undefined) return { found: false };

	const inspection = inspectLocalComponentFile(path, definition);
	return {
		found: true,
		importSource: toImportSource(sourceFile, path),
		importStyle: inspection.importStyle,
		path,
	};
}
