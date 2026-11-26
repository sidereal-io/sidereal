# Image test fixtures

Most test images are built in test code. These few are committed, because
the repository's toolchain cannot write their layout: a baseline JPEG whose
three components are in three separate scans. They were made once with
libjpeg's `cjpeg` and `djpeg`, which the tests do not need.

| File | What it is |
|---|---|
| `jpeg-444-separate-scans.jpg` | A 640 by 480 baseline JPEG, 4:4:4, quality 90, one scan per component. |
| `jpeg-420-separate-scans.jpg` | The same image at 4:2:0 sampling, one scan per component. |
| `jpeg-444-separate-scans.means` | libjpeg's decode of the 4:4:4 file, as the mean of each RGB channel over each 16 by 16 block: 40 × 30 blocks, row by row, 3 bytes per block, rounded. |

## How they were made

1. A 640 by 480 PPM: red `127.5 + 127.5 sin(x / 37)`, green
   `127.5 + 127.5 cos(y / 29)`, and blue
   `(255x / 640 + 255y / 480) / 2`, inverted in alternate 80-pixel squares.
2. A scan script, `scans.txt`, of three lines: `0;`, `1;` and `2;`.
3. `cjpeg -quality 90 -sample 1x1,1x1,1x1 -scans scans.txt` for the 4:4:4
   file, and `-sample 2x2,1x1,1x1` for the 4:2:0 one. `cjpeg` writes a DHT
   segment before the second scan.
4. A comment segment, `between scans`, inserted before the third scan of
   each file, so metadata lies between scans.
5. `djpeg -ppm` of the 4:4:4 file, then the block means.
