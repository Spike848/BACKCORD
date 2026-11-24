/* Showing a picture at the size it's shown: an I420 picture scaled and
 * converted to RGBA through the libyuv inside BACKCORD's libwebrtc archive, into
 * buffers the caller keeps, so no frame needs a buffer of its own. */
#include <stddef.h>

#include "libyuv/convert_argb.h"
#include "libyuv/scale.h"
#include "libyuv/scale_argb.h"

/* The same layout as decode.c's. */
struct backcord_picture {
  int width;
  int height;
  const unsigned char *y;
  const unsigned char *u;
  const unsigned char *v;
  int y_stride;
  int u_stride;
  int v_stride;
};

/* Scales `picture` into `scratch`, an I420 buffer of `width` by `height`,
 * unless it's that size already, and converts it to RGBA in `rgba`, whose
 * rows are `stride` bytes apart. */
void backcord_i420_to_rgba(const struct backcord_picture *picture, unsigned char *scratch,
                       unsigned char *rgba, int stride, int width, int height, int bt709) {
  const unsigned char *y = picture->y, *u = picture->u, *v = picture->v;
  int y_stride = picture->y_stride, u_stride = picture->u_stride, v_stride = picture->v_stride;
  if (picture->width != width || picture->height != height) {
    int chroma = (width + 1) / 2;
    unsigned char *sy = scratch;
    unsigned char *su = sy + (size_t)width * height;
    unsigned char *sv = su + (size_t)chroma * ((height + 1) / 2);
    I420Scale(y, y_stride, u, u_stride, v, v_stride, picture->width, picture->height, sy,
              width, su, chroma, sv, chroma, width, height, kFilterBilinear);
    y = sy;
    u = su;
    v = sv;
    y_stride = width;
    u_stride = chroma;
    v_stride = chroma;
  }
  /* libyuv's ABGR is R, G, B, A in memory; H420 is BT.709's I420. */
  if (bt709)
    H420ToABGR(y, y_stride, u, u_stride, v, v_stride, rgba, stride, width, height);
  else
    I420ToABGR(y, y_stride, u, u_stride, v, v_stride, rgba, stride, width, height);
}

/* BT.709's luma of a pixel, B, G, R in memory, in the limited range. */
static inline unsigned char luma709(const unsigned char *pixel) {
  return (unsigned char)((47 * pixel[2] + 157 * pixel[1] + 16 * pixel[0] + (16 << 8) + 128) >> 8);
}

/* An ARGB picture (B, G, R, A in memory, as captured), `source_width` by
 * `source_height` with rows `source_stride` bytes apart, as I420 of `width`
 * by `height` in BT.709's limited range, which libyuv converts from but not
 * to. A smaller size is scaled first, in ARGB through `scratch` (`width` by
 * `height` by 4 bytes), so the conversion runs over the pixels sent. */
void backcord_argb_to_h420(const unsigned char *argb, int source_stride, int source_width,
                           int source_height, unsigned char *scratch, unsigned char *y,
                           int y_stride, unsigned char *u, int u_stride, unsigned char *v,
                           int v_stride, int width, int height) {
  int stride = source_stride;
  if (source_width != width || source_height != height) {
    ARGBScale(argb, source_stride, source_width, source_height, scratch, width * 4, width, height,
              kFilterBox);
    argb = scratch;
    stride = width * 4;
  }
  for (int row = 0; row < height; row += 2) {
    const unsigned char *top = argb + (size_t)row * stride;
    const unsigned char *bottom = row + 1 < height ? top + stride : top;
    unsigned char *y_top = y + (size_t)row * y_stride;
    unsigned char *y_bottom = row + 1 < height ? y_top + y_stride : y_top;
    unsigned char *u_row = u + (size_t)(row / 2) * u_stride;
    unsigned char *v_row = v + (size_t)(row / 2) * v_stride;
    for (int column = 0; column < width; column += 2) {
      int next = column + 1 < width ? 1 : 0;
      const unsigned char *a = top + column * 4, *b = a + next * 4;
      const unsigned char *c = bottom + column * 4, *d = c + next * 4;
      y_top[column] = luma709(a);
      y_top[column + next] = luma709(b);
      y_bottom[column] = luma709(c);
      y_bottom[column + next] = luma709(d);
      /* Colour from the four pixels' average, four times over. */
      int blue = a[0] + b[0] + c[0] + d[0];
      int green = a[1] + b[1] + c[1] + d[1];
      int red = a[2] + b[2] + c[2] + d[2];
      u_row[column / 2] = (unsigned char)((112 * blue - 86 * green - 26 * red + (128 << 10) + 512) >> 10);
      v_row[column / 2] = (unsigned char)((112 * red - 102 * green - 10 * blue + (128 << 10) + 512) >> 10);
    }
  }
}
