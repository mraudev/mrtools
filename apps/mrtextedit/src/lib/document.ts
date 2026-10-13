/** Saving: the editor HTML as a complete HTML document with the format CSS, so browsers show it as in the editor. */
export function toDocument(body: string, title: string, css: string): string {
  return `<!doctype html>
<html lang="de">
<head>
<meta charset="utf-8">
<title>${escapeHtml(title)}</title>
<style>
${css}
</style>
</head>
<body>
${body}
</body>
</html>
`;
}

/** Opening: the editor content from an HTML document, an HTML fragment or plain text (`.txt`). */
export function fromFile(content: string, path: string): string {
  if (/\.txt$/i.test(path)) {
    return content
      .split(/\r?\n/)
      .map((line) => `<p>${escapeHtml(line)}</p>`)
      .join("");
  }
  if (!/<body[\s>]/i.test(content)) return content;
  return new DOMParser().parseFromString(content, "text/html").body.innerHTML;
}

export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

function escapeHtml(text: string): string {
  return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}
