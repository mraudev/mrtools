// Erhöht die Version einer App, committet und setzt das Tag `<app>-v<version>`.
// Aufruf: npm run release -- <app> <patch|minor|major|x.y.z>
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";

const [app, bump] = process.argv.slice(2);
const run = (cmd, args) => execFileSync(cmd, args, { encoding: "utf8", stdio: ["ignore", "pipe", "inherit"] }).trim();

if (!app || !bump || !existsSync(`apps/${app}/package.json`)) {
  console.error("Aufruf: npm run release -- <app> <patch|minor|major|x.y.z>");
  process.exit(1);
}
if (run("git", ["status", "--porcelain"])) {
  console.error("Es gibt nicht committete Änderungen – erst committen oder verwerfen.");
  process.exit(1);
}

// npm.cmd lässt sich unter Windows nur über die Shell starten.
execFileSync("npm", ["version", bump, "--no-git-tag-version", "-w", `apps/${app}`], { stdio: "inherit", shell: true });
const { version } = JSON.parse(readFileSync(`apps/${app}/package.json`, "utf8"));
const tag = `${app}-v${version}`;

run("git", ["commit", "-am", `${app} ${version}`]);
run("git", ["tag", "-a", tag, "-m", `${app} ${version}`]);
console.log(`\n${tag} angelegt. Veröffentlichen mit:\n  git push --follow-tags`);
