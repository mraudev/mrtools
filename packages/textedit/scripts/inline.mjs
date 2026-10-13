// Builds dist/mrtextedit.html: the embed page with mrtextedit.js inlined – one file for TEdgeBrowser hosts.
import { readFileSync, writeFileSync } from "node:fs";

const dir = new URL("../", import.meta.url);
const js = readFileSync(new URL("dist/mrtextedit.js", dir), "utf8").replaceAll("</script", "<\\/script");
const page = readFileSync(new URL("embed/mrtextedit.html", dir), "utf8");
const tag = '<script src="mrtextedit.js"></script>';
if (!page.includes(tag)) throw new Error(`${tag} fehlt in embed/mrtextedit.html`);
writeFileSync(new URL("dist/mrtextedit.html", dir), page.replace(tag, () => `<script>${js}</script>`));
console.log("dist/mrtextedit.html");
