---
title: 路由验证
description: 验证文章、图标与附件使用同一套直接路径。
tags:
  - 测试
---

# 路由验证

此文章用于 Playwright 路由验收。

```typescript
const articlePath = 'guide';
```

| 文档路径 | 渲染能力 | 布局行为 | 验证范围 |
| --- | --- | --- | --- |
| `projects/asterweft/features/document-workspace` | Markdown、图库与附件 | 内容需要更多横向空间时居中扩展 | 桌面与窄屏视口均不产生页面级横向溢出 |

[打开报告](report.txt)

::gallery
![图库测试图片 01](portrait.svg)
![图库测试图片 02](gallery.svg)
![图库测试图片 03](gallery.svg)
![图库测试图片 04](gallery.svg)
![图库测试图片 05](gallery.svg)
![图库测试图片 06](gallery.svg)
![图库测试图片 07](gallery.svg)
![图库测试图片 08](gallery.svg)
![图库测试图片 09](gallery.svg)
![图库测试图片 10](gallery.svg)
![图库测试图片 11](gallery.svg)
![图库测试图片 12](gallery.svg)
![图库测试图片 13](gallery.svg)
![图库测试图片 14](gallery.svg)
![图库测试图片 15](gallery.svg)
![图库测试图片 16](gallery.svg)
::

::attachments
[report.txt](report.txt)
::
