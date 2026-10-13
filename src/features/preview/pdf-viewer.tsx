import type { ReactNode } from "react";
import { useEffect, useRef, useState } from "react";
import { useServices, useStore } from "@/app/services";
import { Button } from "@/components/button";
import { spawn } from "@/lib/async";
import { PDF_RENDER_CONCURRENCY, SYNCTEX_HIGHLIGHT_MS } from "@/lib/config";
import { errorLog } from "@/lib/log";
import type { PdfDocument, PdfPage, RenderHandle } from "@/lib/pdf";
import { loadPdf } from "@/lib/pdf";
import type { ForwardHit } from "@/lib/tauri";
import { assetUrl } from "@/lib/tauri";

type PageGeometry = {
  readonly number: number;
  readonly widthPt: number;
  readonly heightPt: number;
};

export type Highlight = {
  readonly page: number;
  readonly left: number;
  readonly top: number;
  readonly width: number;
  readonly height: number;
  readonly nonce: number;
};

export type PdfViewerProps = {
  readonly forwardHits: readonly ForwardHit[] | undefined;
  readonly forwardNonce: number;
};

const ZOOM_STEPS = [0.5, 0.67, 0.8, 0.9, 1, 1.1, 1.25, 1.5, 1.75, 2, 2.5, 3];

async function geometryOf(document: PdfDocument): Promise<PageGeometry[]> {
  const pages: PageGeometry[] = [];

  for (let number = 1; number <= document.pageCount; number += 1) {
    const page = await document.page(number);
    pages.push({ number, widthPt: page.widthPt, heightPt: page.heightPt });
  }

  return pages;
}

function PageView(props: {
  readonly document: PdfDocument;
  readonly geometry: PageGeometry;
  readonly scale: number;
  readonly visible: boolean;
  readonly revision: number;
  readonly highlight: Highlight | undefined;
  readonly onInverse: (page: PdfPage, px: number, py: number) => void;
  readonly acquire: () => Promise<() => void>;
}): ReactNode {
  const { document, geometry, scale, visible, revision, highlight, onInverse, acquire } = props;
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const pageRef = useRef<PdfPage | undefined>(undefined);
  const inFlight = useRef<RenderHandle | undefined>(undefined);
  const [rendered, setRendered] = useState<string>("");
  // NOTE: The revision ties the render cache to one loaded PDF; without it a recompiled document
  // with the same page count and scale would never repaint.
  const key = `${revision}:${document.pageCount}:${scale}`;

  useEffect(() => {
    if (!visible || rendered === key) {
      return;
    }

    let cancelled = false;
    const render = async (): Promise<void> => {
      const release = await acquire();

      try {
        const page = await document.page(geometry.number);
        pageRef.current = page;
        const canvas = canvasRef.current;

        if (cancelled || canvas === null) {
          return;
        }

        inFlight.current?.cancel();
        const handle = page.render(canvas, scale, window.devicePixelRatio);
        inFlight.current = handle;
        const finished = await handle.done;

        if (inFlight.current === handle) {
          inFlight.current = undefined;
        }

        if (finished && !cancelled) {
          setRendered(key);
        }
      } finally {
        release();
      }
    };

    spawn(render());

    return () => {
      cancelled = true;
      inFlight.current?.cancel();
    };
  }, [visible, rendered, key, document, geometry.number, scale, acquire]);

  const width = Math.floor(geometry.widthPt * scale);
  const height = Math.floor(geometry.heightPt * scale);

  return (
    <button
      type="button"
      className="pdf-page"
      data-page={geometry.number}
      style={{ width, height }}
      onDoubleClick={(event) => {
        const bounds = event.currentTarget.getBoundingClientRect();
        const page = pageRef.current;

        if (page !== undefined) {
          onInverse(page, event.clientX - bounds.left, event.clientY - bounds.top);
        }
      }}
    >
      <canvas ref={canvasRef} />
      {highlight !== undefined && highlight.page === geometry.number ? (
        <div
          key={highlight.nonce}
          className="synctex-highlight"
          style={{
            left: highlight.left * scale,
            top: highlight.top * scale,
            width: highlight.width * scale,
            height: highlight.height * scale,
          }}
        />
      ) : null}
    </button>
  );
}

export function PdfViewer(props: PdfViewerProps): ReactNode {
  const { forwardHits, forwardNonce } = props;
  const { backend } = useServices();
  const pdf = useStore((state) => state.pdf);
  const compileState = useStore((state) => state.compileState);
  const requestGoTo = useStore((state) => state.requestGoTo);
  const toast = useStore((state) => state.toast);
  const project = useStore((state) => state.project);
  const [document, setDocument] = useState<PdfDocument | undefined>(undefined);
  const [revision, setRevision] = useState(0);
  const [geometry, setGeometry] = useState<readonly PageGeometry[]>([]);
  const [scale, setScale] = useState(1);
  const [fitWidth, setFitWidth] = useState(true);
  const [visible, setVisible] = useState<ReadonlySet<number>>(new Set([1, 2]));
  const [highlight, setHighlight] = useState<Highlight | undefined>(undefined);
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const semaphore = useRef({ active: 0, queue: [] as (() => void)[] });

  const acquire = (): Promise<() => void> =>
    new Promise((resolve) => {
      const release = (): void => {
        semaphore.current.active -= 1;
        const next = semaphore.current.queue.shift();

        if (next !== undefined) {
          next();
        }
      };
      const start = (): void => {
        semaphore.current.active += 1;
        resolve(release);
      };

      if (semaphore.current.active < PDF_RENDER_CONCURRENCY) {
        start();
      } else {
        semaphore.current.queue.push(start);
      }
    });

  useEffect(() => {
    if (pdf === undefined) {
      return;
    }

    let cancelled = false;
    const load = async (): Promise<void> => {
      const next = await loadPdf(`${assetUrl(pdf.path)}?job=${pdf.jobId}`);

      if (cancelled) {
        await next.destroy();

        return;
      }

      const pages = await geometryOf(next);
      const scrollTop = scrollRef.current?.scrollTop ?? 0;
      setDocument((previous) => {
        if (previous !== undefined) {
          spawn(previous.destroy());
        }

        return next;
      });
      setGeometry(pages);
      setRevision(pdf.jobId);
      requestAnimationFrame(() => {
        if (scrollRef.current !== null) {
          scrollRef.current.scrollTop = scrollTop;
        }
      });
    };

    spawn(
      load().catch((rejection: unknown) => {
        errorLog("The PDF could not be loaded.", { rejection });
      }),
    );

    return () => {
      cancelled = true;
    };
  }, [pdf]);

  useEffect(() => {
    const container = scrollRef.current;

    if (container === null || !fitWidth || geometry.length === 0) {
      return;
    }

    const widest = Math.max(...geometry.map((page) => page.widthPt));
    const update = (): void => setScale(Math.max(0.2, (container.clientWidth - 40) / widest));
    update();
    const observer = new ResizeObserver(update);
    observer.observe(container);

    return () => observer.disconnect();
  }, [fitWidth, geometry]);

  useEffect(() => {
    const container = scrollRef.current;

    if (container === null || geometry.length === 0) {
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        setVisible((current) => {
          const next = new Set(current);

          for (const entry of entries) {
            const number = Number((entry.target as HTMLElement).dataset["page"]);

            if (entry.isIntersecting) {
              next.add(number);
              next.add(number + 1);
              next.add(number - 1);
            }
          }

          return next;
        });
      },
      { root: container, rootMargin: "600px 0px" },
    );

    for (const element of container.querySelectorAll<HTMLElement>(".pdf-page")) {
      observer.observe(element);
    }

    return () => observer.disconnect();
  }, [geometry.length]);

  useEffect(() => {
    const hit = forwardHits?.[0];

    if (hit === undefined) {
      return;
    }

    setHighlight({
      page: hit.page,
      left: hit.h,
      top: hit.v - hit.height,
      width: hit.width,
      height: hit.height,
      nonce: forwardNonce,
    });
    const container = scrollRef.current;
    const element = container?.querySelector<HTMLElement>(`.pdf-page[data-page="${hit.page}"]`);

    if (
      container !== null &&
      container !== undefined &&
      element !== null &&
      element !== undefined
    ) {
      container.scrollTo({
        top: element.offsetTop + (hit.v - hit.height) * scale - container.clientHeight / 3,
        behavior: "smooth",
      });
    }

    const timer = setTimeout(() => setHighlight(undefined), SYNCTEX_HIGHLIGHT_MS);

    return () => clearTimeout(timer);
  }, [forwardHits, forwardNonce, scale]);

  const inverse = async (page: PdfPage, px: number, py: number): Promise<void> => {
    const point = page.toPdfPoint(px, py, scale);
    const result = await backend.texSynctexInverse(page.number, point.x, point.y);

    if (!result.ok) {
      toast("warn", `SyncTeX: ${result.error.kind}`);

      return;
    }

    const hit = result.value[0];

    if (hit === undefined || project === undefined) {
      toast("warn", "SyncTeX found no source position here.");

      return;
    }

    const root = `${project.record.root}/`;
    const relative = hit.file.startsWith(root) ? hit.file.slice(root.length) : hit.file;
    requestGoTo(relative, hit.line, Math.max(hit.column, 1));
  };

  const zoom = (direction: 1 | -1): void => {
    setFitWidth(false);
    setScale((current) => {
      const index = ZOOM_STEPS.findIndex((step) => step >= current - 0.001);
      const nextIndex = Math.min(
        Math.max((index === -1 ? ZOOM_STEPS.length - 1 : index) + direction, 0),
        ZOOM_STEPS.length - 1,
      );

      return ZOOM_STEPS[nextIndex] ?? 1;
    });
  };

  return (
    <div className="panel">
      <div className="pdf-toolbar">
        <Button small onClick={() => zoom(-1)} title="Zoom out">
          −
        </Button>
        <span style={{ minWidth: 44, textAlign: "center" }}>{Math.round(scale * 100)}%</span>
        <Button small onClick={() => zoom(1)} title="Zoom in">
          +
        </Button>
        <Button small variant={fitWidth ? "primary" : "default"} onClick={() => setFitWidth(true)}>
          Fit width
        </Button>
        <span className="spacer" style={{ flex: 1 }} />
        <span className="hint">
          {compileState === "running"
            ? "Compiling…"
            : geometry.length > 0
              ? `${geometry.length} pages`
              : ""}
        </span>
      </div>
      <div className="pdf-viewer" ref={scrollRef}>
        {document === undefined ? (
          <div className="empty">
            {pdf === undefined ? "Compile to see the PDF here." : "Loading PDF…"}
          </div>
        ) : (
          <div className="pdf-pages">
            {geometry.map((page) => (
              <PageView
                key={page.number}
                document={document}
                geometry={page}
                scale={scale}
                visible={visible.has(page.number)}
                revision={revision}
                highlight={highlight}
                onInverse={(target, px, py) => spawn(inverse(target, px, py))}
                acquire={acquire}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
