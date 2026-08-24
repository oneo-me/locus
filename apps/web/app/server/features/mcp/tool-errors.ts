interface McpToolErrorDefinition {
  code: string;
  message: string;
  retryable: boolean;
}

const fallbackError: McpToolErrorDefinition = {
  code: 'OPERATION_FAILED',
  message: 'The operation could not be completed.',
  retryable: false,
};

function classifyToolError(error: unknown): McpToolErrorDefinition {
  const message = error instanceof Error ? error.message : String(error);

  if (message.includes('文章已发生变化')) {
    return {
      code: 'REVISION_CONFLICT',
      message:
        'The article changed since it was read. Read it again and retry with the latest revision.',
      retryable: true,
    };
  }
  if (message.includes('文章包含子文章')) {
    return {
      code: 'ARTICLE_HAS_CHILDREN',
      message:
        'The article has child articles. Set recursive to true to delete the entire subtree.',
      retryable: false,
    };
  }
  if (message.includes('文章已经存在') || message.includes('文档已经存在')) {
    return {
      code: 'ARTICLE_ALREADY_EXISTS',
      message: 'An article already exists at this path.',
      retryable: false,
    };
  }
  if (
    message.includes('文章不存在') ||
    message.includes('根文章不存在') ||
    message.includes('文档不存在')
  ) {
    return {
      code: 'ARTICLE_NOT_FOUND',
      message: 'The requested article does not exist.',
      retryable: false,
    };
  }
  if (message.includes('附件已经存在')) {
    return {
      code: 'ATTACHMENT_ALREADY_EXISTS',
      message:
        'An attachment with this name already exists. Set overwrite to true to replace it.',
      retryable: false,
    };
  }
  if (
    message.includes('不是该文章的附件') ||
    message.includes('不是该文档的附件') ||
    message.includes('附件不存在或无法访问')
  ) {
    return {
      code: 'ATTACHMENT_NOT_FOUND',
      message: 'The requested attachment does not exist for this article.',
      retryable: false,
    };
  }
  if (
    message.includes('同名路径不是可覆盖的附件') ||
    message.includes('文章正文文件') ||
    message.includes('文档库缓存文件') ||
    message.includes('受保护的 icon.png')
  ) {
    return {
      code: 'PROTECTED_RESOURCE',
      message:
        'The requested path is protected and cannot be used as an attachment.',
      retryable: false,
    };
  }
  if (
    message.includes('不能为空') ||
    message.includes('必须是') ||
    message.includes('至少需要') ||
    message.includes('offset') ||
    message.includes('maxBytes') ||
    message.includes('maxChars') ||
    message.includes('一次最多') ||
    message.includes('新文章路径必须与当前路径不同') ||
    message.includes('不能将文章移动到自己的子文章路径中') ||
    message.includes('不能移动文档库根文章的路径') ||
    message.includes('新文章路径不能是文档库虚拟根路径') ||
    message.includes('不支持的文章更新操作')
  ) {
    return {
      code: 'INVALID_REQUEST',
      message:
        'The request contains an invalid value or unsupported operation.',
      retryable: false,
    };
  }
  if (
    message.includes('无法安全') ||
    message.includes('超出文档库范围') ||
    message.includes('目录无法安全访问') ||
    message.includes('文章路径无法安全访问') ||
    message.includes('附件存储位置无法安全访问')
  ) {
    return {
      code: 'UNSAFE_PATH',
      message: 'The requested path cannot be accessed safely.',
      retryable: false,
    };
  }
  if (
    message.includes('无法创建文章') ||
    message.includes('无法更新文章') ||
    message.includes('无法移动文章') ||
    message.includes('无法写入附件') ||
    message.includes('无法索引附件')
  ) {
    return {
      code: 'WRITE_FAILED',
      message: 'The requested change could not be written.',
      retryable: true,
    };
  }
  return fallbackError;
}

export function toolErrorResult(error: unknown) {
  const definition = classifyToolError(error);
  const value = {
    error: {
      code: definition.code,
      message: definition.message,
      retryable: definition.retryable,
    },
  };
  return {
    isError: true as const,
    content: [{ type: 'text' as const, text: JSON.stringify(value, null, 2) }],
    structuredContent: value,
  };
}

export async function handleMcpToolCall<T>(
  operation: () => T | Promise<T>,
): Promise<T | ReturnType<typeof toolErrorResult>> {
  try {
    return await operation();
  } catch (error) {
    return toolErrorResult(error);
  }
}
