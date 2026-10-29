# Scanned image rendering reproduction for issue 2256

I rendered pages 1, 2, 50, and 120 of the reporter's linked sample file at 96, 144, and 192 dpi with the PdfCraft CLI, MuPDF, and Poppler. I compared each render with an area-averaged reference made from the decoded embedded image, resized to the output dimensions. RMS error is normalized to the 0–1 range (8-bit RGB divided by 255).

The file has 242 pages at 377.76 × 543.84 pt. The sampled image encodings are mixed: page 1 is a 3148 × 4532 DCT RGB image; pages 2, 50, and 120 are 3148 × 4532 1-bit JBIG2 images.

| Page | DPI | PdfCraft RMS | MuPDF RMS | Poppler RMS |
|---:|---:|---:|---:|---:|
| 50 | 96 | 0.185675 | 0.036280 | 0.035665 |
| 120 | 96 | 0.189592 | 0.036197 | 0.035857 |
| 50 | 144 | 0.103305 | 0.048967 | 0.046604 |
| 120 | 144 | 0.103163 | 0.049490 | 0.047026 |

Visual inspection of page 50 at 96 dpi shows noticeably more jagged, uneven scan strokes in the PdfCraft output. These measurements establish a difference for this sample and the tested raster sizes; they do not establish the exact behavior at a live display zoom.

The source path is consistent with that result. In `vendor/hayro/src/renderer.rs`, the pre-minification pass chooses nearest-neighbour when `/Interpolate` is false; that is the PDF default. Point sampling can drop thin strokes when shrinking scans. The GUI queues its visible page pixel requests through the same `PageRenderer` render pool as the CLI (`crates/ui-egui/src/canvas/cache.rs`), so it uses the same raster path. This GUI conclusion is from reading the call path; the GUI itself was not launched.

I also read issue 350 and checked open image-minification work: [PR 3858](https://github.com/storytold/pdfcraft/pull/3858), [PR 3683](https://github.com/storytold/pdfcraft/pull/3683), and [PR 1586](https://github.com/storytold/pdfcraft/pull/1586). The renders, decoded source images, complete metrics, and the helper used to calculate them are retained with the local reproduction evidence; no sample document or image is committed here.

Not run: the GUI itself, Windows, macOS, or comparison at real display zoom levels.
