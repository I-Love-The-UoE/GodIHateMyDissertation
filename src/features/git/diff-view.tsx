import type { ReactNode } from "react";
import { useEffect, useRef } from "react";
import { createMergeView } from "@/lib/codemirror";
import type { FileDiff } from "@/lib/tauri";

export function DiffView(props: { readonly diff: FileDiff }): ReactNode {
  const { diff } = props;
  const hostRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const host = hostRef.current;

    if (host === null || diff.binary) {
      return;
    }

    const handle = createMergeView(host, diff.oldText, diff.newText);

    return () => handle.destroy();
  }, [diff]);

  if (diff.binary) {
    return <div className="empty">Binary file.</div>;
  }

  return <div className="diff-host" ref={hostRef} />;
}
