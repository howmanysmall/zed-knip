#!/usr/bin/env bun

import { type } from "arktype";
import { file } from "bun";

const GITIGNORE = file(".gitignore");
const SKILLS_DIR = ".agents/skills";

const isLockFile = type({
	skills: "Record<string, unknown>",
	version: "1",
});

const lockfile = isLockFile.assert(await file("skills-lock.json").json());

const skillNames = Object.keys(lockfile.skills);
const lines = skillNames.map((name) => `${SKILLS_DIR}/${name}/`);

let gitignore = await GITIGNORE.text();
const existingLines = new Set(gitignore.split("\n").map((line) => line.trim()));

let added = 0;
for (const line of lines) {
	if (!existingLines.has(line)) {
		gitignore += `${line}\n`;
		added += 1;
	}
}

const { consola } = await import("consola");
if (added > 0) {
	await GITIGNORE.write(gitignore);
	consola.success(`Added ${added} skill(s) to .gitignore`);
} else consola.info("All skills already in .gitignore");
