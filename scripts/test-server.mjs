import { spawn } from 'node:child_process';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
const directory = await mkdtemp(join(tmpdir(), 'open-ledger-e2e-'));
const target = resolve(process.env.CARGO_TARGET_DIR || 'target');
const child = spawn(
  join(target, 'debug', process.platform === 'win32' ? 'ledger-server.exe' : 'ledger-server'),
  ['--data', join(directory, 'books.sqlite'), '--port', '4178'],
  { stdio: 'inherit' },
);
let stopping = false;
const stop = () => {
  stopping = true;
  child.kill();
};
process.on('SIGTERM', stop);
process.on('SIGINT', stop);
child.on('error', async (error) => {
  console.error(error);
  await rm(directory, { recursive: true, force: true });
  process.exit(1);
});
child.on('exit', async (code) => {
  await rm(directory, { recursive: true, force: true });
  process.exit(stopping ? 0 : (code ?? 1));
});
