import { useState } from 'react';
import { useCanvas } from '@locus/canvas';

export default function Canvas() {
  const { note } = useCanvas();
  const [count, setCount] = useState(0);
  return <main className="min-h-screen bg-white p-6 text-zinc-900 dark:bg-zinc-950 dark:text-zinc-100">
    <p className="mb-4">{note}</p>
    <button className="rounded border px-3 py-2" onClick={() => setCount(count + 1)}>
      Count: {count}
    </button>
  </main>;
}
