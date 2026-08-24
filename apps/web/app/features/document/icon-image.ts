import {
  documentIconSize,
  maxDocumentIconBytes,
} from '~/contracts/document-icon';

const maxSourceImageBytes = 16 * 1024 * 1024;

export async function processDocumentIconFile(file: File): Promise<string> {
  if (!file.type.startsWith('image/')) {
    throw new Error('请选择图片文件。');
  }
  if (file.size > maxSourceImageBytes) {
    throw new Error('原始图片不能超过 16 MiB。');
  }

  let bitmap: ImageBitmap;
  try {
    bitmap = await createImageBitmap(file);
  } catch {
    throw new Error('无法读取这张图片。');
  }

  try {
    if (bitmap.width < 1 || bitmap.height < 1) {
      throw new Error('图片尺寸无效。');
    }
    const canvas = document.createElement('canvas');
    canvas.width = documentIconSize;
    canvas.height = documentIconSize;
    const context = canvas.getContext('2d');
    if (!context) throw new Error('浏览器无法处理这张图片。');

    const sourceSize = Math.min(bitmap.width, bitmap.height);
    const sourceX = (bitmap.width - sourceSize) / 2;
    const sourceY = (bitmap.height - sourceSize) / 2;
    context.imageSmoothingEnabled = true;
    context.imageSmoothingQuality = 'high';
    context.drawImage(
      bitmap,
      sourceX,
      sourceY,
      sourceSize,
      sourceSize,
      0,
      0,
      documentIconSize,
      documentIconSize,
    );

    const blob = await new Promise<Blob>((resolve, reject) => {
      canvas.toBlob(
        (result) =>
          result
            ? resolve(result)
            : reject(new Error('浏览器无法生成 PNG 图片。')),
        'image/png',
      );
    });
    if (blob.size > maxDocumentIconBytes) {
      throw new Error('处理后的图标不能超过 256 KiB。');
    }
    return bytesToBase64(new Uint8Array(await blob.arrayBuffer()));
  } finally {
    bitmap.close();
  }
}

function bytesToBase64(data: Uint8Array): string {
  let binary = '';
  for (const byte of data) binary += String.fromCharCode(byte);
  return btoa(binary);
}
