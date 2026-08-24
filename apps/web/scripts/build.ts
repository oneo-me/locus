import { existsSync, renameSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { buildProduction, projectRoot } from './build-production';

const timestamp = Date.now();
const outputDirectory = join(projectRoot, 'build');
const stagingDirectory = join(
  projectRoot,
  `.build-stage-${process.pid}-${timestamp}`,
);
const previousDirectory = join(
  projectRoot,
  `.build-previous-${process.pid}-${timestamp}`,
);

try {
  const log = await buildProduction(stagingDirectory);
  if (log) console.log(log);
  if (existsSync(outputDirectory)) {
    renameSync(outputDirectory, previousDirectory);
  }
  renameSync(stagingDirectory, outputDirectory);
  rmSync(previousDirectory, { recursive: true, force: true });
  console.log('生产构建已写入 build/client 与 build/server。');
} catch (error) {
  rmSync(stagingDirectory, { recursive: true, force: true });
  if (!existsSync(outputDirectory) && existsSync(previousDirectory)) {
    renameSync(previousDirectory, outputDirectory);
  }
  throw error;
}
