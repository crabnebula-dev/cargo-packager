import fs from "fs";
import path from "path";
import { promisify } from "util";
import { pipeline as streamPipeline } from "stream";
import yauzl from "yauzl";

const pipeline = promisify(streamPipeline);

const IFMT = 0o170000;
const IFDIR = 0o040000;
const IFLNK = 0o120000;
const MAX_SYMLINK_TARGET_SIZE = 4096;
const MAX_SYMLINK_HOPS = 40;

interface SymlinkEntry {
  name: string;
  target: string;
}

function validateArchiveName(name: string): void {
  const error = yauzl.validateFileName(name);

  if (error !== null) {
    throw new Error(`Unsafe ZIP entry name "${name}": ${error}`);
  }

  if (name.includes("\0") || name.includes("\\")) {
    throw new Error(`Unsafe ZIP entry name "${name}"`);
  }

  if (/^(?:[a-zA-Z]:|\/\/)/.test(name)) {
    throw new Error(`Absolute ZIP entry name "${name}" is not allowed`);
  }
}

function validateSymlinkTarget(name: string, target: string): void {
  if (!target || target.includes("\0") || target.includes("\\")) {
    throw new Error(`Unsafe symlink target "${target}" for "${name}"`);
  }

  if (path.posix.isAbsolute(target) || /^(?:[a-zA-Z]:|\/\/)/.test(target)) {
    throw new Error(`Absolute symlink target "${target}" is not allowed`);
  }

  const resolved = path.posix.normalize(
    path.posix.join(path.posix.dirname(name), target),
  );

  if (resolved === ".." || resolved.startsWith("../")) {
    throw new Error(
      `Symlink "${name}" points outside the extraction directory`,
    );
  }
}

function archiveToDiskPath(root: string, archiveName: string): string {
  const relative = archiveName.split("/").join(path.sep);
  const destination = path.resolve(root, relative);
  const relativeToRoot = path.relative(root, destination);

  if (
    relativeToRoot === ".." ||
    relativeToRoot.startsWith(`..${path.sep}`) ||
    path.isAbsolute(relativeToRoot)
  ) {
    throw new Error(`ZIP entry "${archiveName}" escapes extraction directory`);
  }

  return destination;
}

async function lstatIfExists(filePath: string): Promise<fs.Stats | null> {
  try {
    return await promisify(fs.lstat)(filePath);
  } catch (error: any) {
    if (error.code === "ENOENT") {
      return null;
    }

    throw error;
  }
}

async function ensureDirectory(root: string, archiveName: string): Promise<void> {
  const parts = archiveName.split("/").filter(Boolean);
  let current = root;

  for (const part of parts) {
    current = path.join(current, part);

    const existing = await lstatIfExists(current);

    if (existing) {
      if (existing.isSymbolicLink() || !existing.isDirectory()) {
        throw new Error(`Unsafe directory path "${archiveName}"`);
      }
    } else {
      await promisify(fs.mkdir)(current);
    }
  }
}

async function readSymlinkTarget(
  zipFile: yauzl.ZipFile,
  entry: yauzl.Entry,
): Promise<string> {
  if (entry.uncompressedSize > MAX_SYMLINK_TARGET_SIZE) {
    throw new Error(`Symlink target for "${entry.fileName}" is too large`);
  }

  const stream = await promisify(
    zipFile.openReadStream.bind(zipFile),
  )(entry);

  const chunks: Buffer[] = [];
  let size = 0;

  await new Promise<void>((resolve, reject) => {
    stream.on("data", (chunk: Buffer) => {
      size += chunk.length;

      if (size > MAX_SYMLINK_TARGET_SIZE) {
        stream.destroy(
          new Error(`Symlink target for "${entry.fileName}" is too large`),
        );
        return;
      }

      chunks.push(chunk);
    });

    stream.on("end", resolve);
    stream.on("error", reject);
  });

  return Buffer.concat(chunks).toString("utf8");
}

function resolveSymlinkPath(
  name: string,
  symlinks: Map<string, string>,
): string {
  let current = name;
  const visited = new Set<string>();

  for (let hop = 0; hop < MAX_SYMLINK_HOPS; hop++) {
    const parts = current.split("/").filter(Boolean);
    let prefix = "";
    let followed = false;

    for (let i = 0; i < parts.length; i++) {
      prefix = prefix ? `${prefix}/${parts[i]}` : parts[i];

      const target = symlinks.get(prefix);

      if (target === undefined) {
        continue;
      }

      if (visited.has(prefix)) {
        throw new Error(`Symlink loop detected involving "${prefix}"`);
      }

      visited.add(prefix);

      const targetPath = path.posix.normalize(
        path.posix.join(path.posix.dirname(prefix), target),
      );

      if (targetPath === ".." || targetPath.startsWith("../")) {
        throw new Error(`Symlink "${prefix}" escapes extraction directory`);
      }

      const remainder = parts.slice(i + 1).join("/");
      current = remainder
        ? path.posix.join(targetPath, remainder)
        : targetPath;

      followed = true;
      break;
    }

    if (!followed) {
      return current;
    }
  }

  throw new Error(`Symlink chain is too deep for "${name}"`);
}

async function createSymlink(
  root: string,
  entry: SymlinkEntry,
  symlinks: Map<string, string>,
): Promise<void> {
  validateSymlinkTarget(entry.name, entry.target)

const initialTarget = path.posix.normalize(
  path.posix.join(path.posix.dirname(entry.name), entry.target),
);

const resolved = resolveSymlinkPath(initialTarget, symlinks);

  if (resolved === ".." || resolved.startsWith("../")) {
    throw new Error(
      `Symlink "${entry.name}" resolves outside extraction directory`,
    );
  }

  const destination = archiveToDiskPath(root, entry.name);
  const existing = await lstatIfExists(destination);

  if (existing) {
    throw new Error(
      `Symlink "${entry.name}" conflicts with an existing archive entry`,
    );
  }

  const parent = path.dirname(destination);
  const parentRelative = path.relative(root, parent);

  if (parentRelative) {
    await ensureDirectory(root, parentRelative.split(path.sep).join("/"));
  }

  await promisify(fs.symlink)(entry.target, destination);
}

function openZip(zipPath: string): Promise<yauzl.ZipFile> {
  return new Promise((resolve, reject) => {
    yauzl.open(
      zipPath,
      {
        lazyEntries: true,
        validateEntrySizes: true,
        strictFileNames: true,
        autoClose: false,
      },
      (error, zipFile) => {
        if (error) {
          reject(error);
        } else {
          resolve(zipFile);
        }
      },
    );
  });
}

export async function extractZip(
  zipPath: string,
  destination: string,
): Promise<void> {
  if (!path.isAbsolute(destination)) {
    throw new TypeError("Extraction directory must be an absolute path");
  }

  const root = path.resolve(destination);
  const rootStats = await lstatIfExists(root);

  if (!rootStats || !rootStats.isDirectory() || rootStats.isSymbolicLink()) {
    throw new Error("Extraction directory must be a real directory");
  }

  const zipFile = await openZip(zipPath);
  const symlinks: SymlinkEntry[] = [];
  const symlinkMap = new Map<string, string>();
  const entries = new Set<string>();

  try {
    await new Promise<void>((resolve, reject) => {
      let finished = false;

      const fail = (error: Error) => {
        if (!finished) {
          finished = true;
          reject(error);
        }
      };

      zipFile.on("error", fail);

      zipFile.on("end", () => {
        if (!finished) {
          finished = true;
          resolve();
        }
      });

      zipFile.on("entry", async (entry) => {
        try {
          const isDirectory =
            entry.fileName.endsWith("/") ||
            (((entry.externalFileAttributes >>> 16) & IFMT) === IFDIR);

          const mode = (entry.externalFileAttributes >>> 16) & 0xffff;
          const isSymlink = (mode & IFMT) === IFLNK;

          const archiveName = entry.fileName.replace(/\/+$/, "");

          if (!archiveName) {
            zipFile.readEntry();
            return;
          }

          validateArchiveName(archiveName);

          if (entries.has(archiveName)) {
            throw new Error(`Duplicate ZIP entry "${archiveName}"`);
          }

          entries.add(archiveName);

          if (isSymlink) {
            const target = await readSymlinkTarget(zipFile, entry);

            validateSymlinkTarget(archiveName, target);

            symlinks.push({
              name: archiveName,
              target,
            });

            symlinkMap.set(archiveName, target);
          } else if (isDirectory) {
            await ensureDirectory(root, archiveName);
          } else {
            const destinationPath = archiveToDiskPath(root, archiveName);
            const parent = path.posix.dirname(archiveName);

            if (parent && parent !== ".") {
              await ensureDirectory(root, parent);
            }

            const existing = await lstatIfExists(destinationPath);

            if (existing) {
              throw new Error(
                `ZIP entry "${archiveName}" conflicts with an existing file`,
              );
            }

            const fileMode = mode === 0 ? 0o666 : mode & 0o777;

            const output = fs.createWriteStream(destinationPath, {
              flags: "wx",
              mode: fileMode,
            });

            const input = await promisify(
              zipFile.openReadStream.bind(zipFile),
            )(entry);

            await pipeline(input, output);
          }

          zipFile.readEntry();
        } catch (error) {
          fail(error as Error);
        }
      });

      zipFile.readEntry();
    });

    for (const entry of symlinks) {
      await createSymlink(root, entry, symlinkMap);
    }
  } finally {
    zipFile.close();
  }
}

export default extractZip;
