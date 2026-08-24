---
title: 画布
description: 验证 HTML 片段使用项目内置 Tailwind 运行时按需渲染并执行交互。
tags:
  - 测试
---

# 画布

下方画布直接显示在笔记中，并突破正文栏占满笔记主区域。Tailwind 样式生效，作者脚本可以响应交互。

## 阅读上下文

- 保留第一段阅读位置。
- 保留第二段阅读位置。
- 保留第三段阅读位置。
- 保留第四段阅读位置。
- 保留第五段阅读位置。
- 保留第六段阅读位置。
- 保留第七段阅读位置。
- 保留第八段阅读位置。
- 保留第九段阅读位置。
- 保留第十段阅读位置。
- 保留第十一段阅读位置。
- 保留第十二段阅读位置。
- 保留第十三段阅读位置。
- 保留第十四段阅读位置。
- 保留第十五段阅读位置。
- 保留第十六段阅读位置。
- 保留第十七段阅读位置。
- 保留第十八段阅读位置。
- 保留第十九段阅读位置。
- 保留第二十段阅读位置。

::canvas
```html
<main class="h-[640px] bg-sky-50 p-8 text-slate-900 dark:bg-slate-950 dark:text-slate-100">
  <div class="mx-auto max-w-xl">
    <h1 class="text-2xl font-semibold">嵌入式页面</h1>
    <p class="mt-2 text-slate-600 dark:text-slate-300">HTML 片段使用内置 Tailwind 运行时渲染。</p>
    <div class="mt-4 size-8 bg-[#ff007f]" data-export-marker></div>
    <button
      class="mt-5 rounded-md bg-sky-700 px-4 py-2 font-medium text-white dark:bg-sky-500 dark:text-slate-950"
      id="canvas-counter"
      onclick="this.dataset.inlineHandler = 'true'"
      type="button"
    >
      点击 0 次
    </button>
  </div>
</main>
<script>
  try {
    window.parent.document.documentElement.dataset.canvasCompromised = 'true';
  } catch {}

  document.body.dataset.scriptRan = 'true';
  const counter = document.querySelector('#canvas-counter');
  let count = 0;
  counter.addEventListener('click', () => {
    count += 1;
    counter.textContent = `点击 ${count} 次`;
  });
</script>
```
::

::canvas
```html
<section class="flex h-[240px] items-center gap-5 bg-slate-100 p-6 text-slate-900 dark:bg-slate-900 dark:text-slate-100">
  <img class="size-28 rounded-2xl" src="canvas-attachment.svg" alt="画布附件示例" />
  <div>
    <h2 class="text-lg font-semibold">自动高度画布</h2>
    <p class="mt-1 text-sm text-slate-600 dark:text-slate-300">图片来自当前笔记的附件。</p>
  </div>
</section>
```
::

::canvas
```html
<main class="bg-slate-100 p-4 text-slate-900 sm:p-6">
  <div class="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
    <article class="h-24 rounded-2xl border border-slate-200 bg-white p-4 shadow-sm"><span class="block size-4 bg-[#a855f7]"></span></article>
    <article class="h-24 rounded-2xl border border-slate-200 bg-white p-4 shadow-sm"><span class="block size-4 bg-[#a855f7]"></span></article>
    <article class="h-24 rounded-2xl border border-slate-200 bg-white p-4 shadow-sm"><span class="block size-4 bg-[#a855f7]"></span></article>
    <article class="h-24 rounded-2xl border border-slate-200 bg-white p-4 shadow-sm"><span class="block size-4 bg-[#a855f7]"></span></article>
    <article class="h-24 rounded-2xl border border-slate-200 bg-white p-4 shadow-sm"><span class="block size-4 bg-[#a855f7]"></span></article>
    <article class="h-24 rounded-2xl border border-slate-200 bg-white p-4 shadow-sm"><span class="block size-4 bg-[#a855f7]"></span></article>
  </div>
</main>
```
::
