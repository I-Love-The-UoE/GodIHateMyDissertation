import type { PDFDocumentProxy, PDFPageProxy } from "pdfjs-dist";
import * as pdfjs from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";

pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;

const LOAD_OPTIONS = {
  wasmUrl: "/pdfjs/wasm/",
  cMapUrl: "/pdfjs/cmaps/",
  cMapPacked: true,
  standardFontDataUrl: "/pdfjs/standard_fonts/",
} as const;

export type PdfDocument = {
  readonly pageCount: number;
  readonly page: (number: number) => Promise<PdfPage>;
  readonly destroy: () => Promise<void>;
};

export type RenderHandle = {
  /** Resolves to false when the render was cancelled before it finished. */
  readonly done: Promise<boolean>;
  readonly cancel: () => void;
};

export type PdfPage = {
  readonly number: number;
  /** Width and height in PDF points at scale 1. */
  readonly widthPt: number;
  readonly heightPt: number;
  readonly render: (
    canvas: HTMLCanvasElement,
    scale: number,
    devicePixelRatio: number,
  ) => RenderHandle;
  /** Converts canvas pixel coordinates (top-left origin) to PDF points with a top-left origin. */
  readonly toPdfPoint: (
    px: number,
    py: number,
    scale: number,
  ) => { readonly x: number; readonly y: number };
};

function wrapPage(proxy: PDFPageProxy): PdfPage {
  const base = proxy.getViewport({ scale: 1 });

  return {
    number: proxy.pageNumber,
    widthPt: base.width,
    heightPt: base.height,
    render: (canvas, scale, devicePixelRatio) => {
      const viewport = proxy.getViewport({ scale: scale * devicePixelRatio });
      canvas.width = Math.floor(viewport.width);
      canvas.height = Math.floor(viewport.height);
      canvas.style.width = `${Math.floor(viewport.width / devicePixelRatio)}px`;
      canvas.style.height = `${Math.floor(viewport.height / devicePixelRatio)}px`;
      const context = canvas.getContext("2d");

      if (context === null) {
        return { done: Promise.resolve(false), cancel: () => undefined };
      }

      const task = proxy.render({ canvas, canvasContext: context, viewport });
      // NOTE: pdf.js rejects a cancelled render with RenderingCancelledException; that is the
      // expected outcome of cancel(), not a failure.
      const done = task.promise.then(
        () => true,
        (rejection: unknown) => {
          if (rejection instanceof pdfjs.RenderingCancelledException) {
            return false;
          }

          throw rejection;
        },
      );

      return { done, cancel: () => task.cancel() };
    },
    toPdfPoint: (px, py, scale) => {
      const viewport = proxy.getViewport({ scale });
      const [x, yBottomOrigin] = viewport.convertToPdfPoint(px, py);

      return { x: x ?? 0, y: base.height - (yBottomOrigin ?? 0) };
    },
  };
}

function wrapDocument(proxy: PDFDocumentProxy): PdfDocument {
  const pages = new Map<number, Promise<PdfPage>>();

  return {
    pageCount: proxy.numPages,
    page: (number) => {
      const cached = pages.get(number);

      if (cached !== undefined) {
        return cached;
      }

      const loading = proxy.getPage(number).then(wrapPage);
      pages.set(number, loading);

      return loading;
    },
    destroy: () => proxy.loadingTask.destroy(),
  };
}

export async function loadPdf(url: string): Promise<PdfDocument> {
  const response = await fetch(url);
  const bytes = new Uint8Array(await response.arrayBuffer());
  const proxy = await pdfjs.getDocument({ data: bytes, ...LOAD_OPTIONS }).promise;

  return wrapDocument(proxy);
}
