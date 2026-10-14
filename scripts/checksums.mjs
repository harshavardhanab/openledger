import { readdir, readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { join } from 'node:path';
const directory = process.argv[2] || 'artifacts';
const names = (await readdir(directory))
  .filter((n) => !n.endsWith('.asc') && n !== 'SHA256SUMS')
  .sort();
const lines = [];
for (const name of names) {
  lines.push(
    `${createHash('sha256')
      .update(await readFile(join(directory, name)))
      .digest('hex')}  ${name}`,
  );
}
await writeFile(join(directory, 'SHA256SUMS'), lines.join('\n') + '\n');
