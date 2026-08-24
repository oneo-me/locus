export const documentIconName = 'icon.png';
export const documentIconSize = 64;
export const maxDocumentIconBytes = 256 * 1024;

const pngSignature = [137, 80, 78, 71, 13, 10, 26, 10] as const;

export function isDocumentIconName(name: string): boolean {
  return name.toLowerCase() === documentIconName;
}

export function assertValidDocumentIcon(data: Uint8Array): void {
  if (data.byteLength > maxDocumentIconBytes) {
    throw new Error('文档图标不能超过 256 KiB。');
  }
  if (
    data.byteLength < 45 ||
    pngSignature.some((value, index) => data[index] !== value)
  ) {
    throw new Error('文档图标必须是有效的 PNG 图片。');
  }

  const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
  let offset = 8;
  let chunkIndex = 0;
  let foundEnd = false;
  while (offset + 12 <= data.byteLength) {
    const length = view.getUint32(offset);
    const type = readAscii(data, offset + 4, 4);
    const nextOffset = offset + 12 + length;
    if (nextOffset > data.byteLength) {
      throw new Error('文档图标必须是有效的 PNG 图片。');
    }
    if (chunkIndex === 0 && (type !== 'IHDR' || length !== 13)) {
      throw new Error('文档图标必须是有效的 PNG 图片。');
    }
    if (type === 'IEND') {
      if (length !== 0 || nextOffset !== data.byteLength) {
        throw new Error('文档图标必须是有效的 PNG 图片。');
      }
      foundEnd = true;
      break;
    }
    offset = nextOffset;
    chunkIndex += 1;
  }
  if (!foundEnd) throw new Error('文档图标必须是有效的 PNG 图片。');

  if (
    view.getUint32(16) !== documentIconSize ||
    view.getUint32(20) !== documentIconSize
  ) {
    throw new Error('文档图标必须是 64×64 像素。');
  }
}

function readAscii(data: Uint8Array, offset: number, length: number): string {
  return String.fromCharCode(...data.subarray(offset, offset + length));
}
