/* Cameras through libwebrtc's video capture module, inside BACKCORD's libwebrtc
 * archive: V4L2 on Linux, and DirectShow on Windows. Frames arrive raw from
 * the device (YUY2, MJPEG, NV12 and so on), on the module's own capture
 * thread, and come out as I420 planes in buffers this camera reuses for every
 * frame. */
#include <stdint.h>
#include <string.h>

#include <algorithm>
#include <array>
#include <memory>
#include <vector>

#include "libyuv/convert.h"
#include "libyuv/scale.h"
#include "modules/video_capture/raw_video_sink_interface.h"
#include "modules/video_capture/video_capture.h"
#include "modules/video_capture/video_capture_factory.h"

extern "C" {

/* The same layout as decode.c's: planes valid until the callback returns. */
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

typedef void (*backcord_camera_frame)(void *user, const struct backcord_picture *picture);

}  // extern "C"

namespace {

/* An I420 picture of one size, allocated once. */
struct Planes {
  int width = 0;
  int height = 0;
  std::vector<uint8_t> data;

  void resize(int w, int h) {
    if (w == width && h == height) return;
    width = w;
    height = h;
    data.assign((size_t)w * h + 2 * (size_t)((w + 1) / 2) * ((h + 1) / 2), 0);
  }
  uint8_t *y() { return data.data(); }
  uint8_t *u() { return y() + (size_t)width * height; }
  uint8_t *v() { return u() + (size_t)((width + 1) / 2) * ((height + 1) / 2); }
  int chroma_stride() const { return (width + 1) / 2; }

  backcord_picture picture() {
    return backcord_picture{width, height, y(), u(), v(), width, chroma_stride(), chroma_stride()};
  }

  /* JPEG's full range (0-255) brought into the limited range (16-235, and
   * 16-240 for colour) every other format comes in and the encoders
   * announce, so a camera's colours stay the same whichever format its
   * resolution takes: webcams send their larger sizes as MJPEG, which
   * libyuv decodes without changing the range. */
  void limit_range() {
    static const std::array<std::array<uint8_t, 256>, 2> tables = [] {
      std::array<std::array<uint8_t, 256>, 2> made{};
      for (int value = 0; value < 256; value++) {
        made[0][value] = (uint8_t)(16 + (value * 219 + 127) / 255);
        int chroma = (value - 128) * 224;
        made[1][value] = (uint8_t)(128 + (chroma + (chroma >= 0 ? 127 : -127)) / 255);
      }
      return made;
    }();
    size_t luma = (size_t)width * height;
    for (size_t at = 0; at < luma; at++) data[at] = tables[0][data[at]];
    for (size_t at = luma; at < data.size(); at++) data[at] = tables[1][data[at]];
  }
};

class Sink : public webrtc::RawVideoSinkInterface {
 public:
  Sink(int max_width, int max_height, backcord_camera_frame frame, void *user)
      : max_width_(max_width), max_height_(max_height), frame_(frame), user_(user) {}

  int32_t OnRawFrame(uint8_t *data, size_t length,
                     const webrtc::VideoCaptureCapability &info,
                     webrtc::VideoRotation, int64_t) override {
    int width = info.width;
    int height = std::abs(info.height);
    if (width <= 0 || height <= 0) return -1;
    converted_.resize(width, height);
    // Devices fill in the size of formats such as YUY2 themselves.
    int failed = libyuv::ConvertToI420(
        data, length, converted_.y(), converted_.width, converted_.u(),
        converted_.chroma_stride(), converted_.v(), converted_.chroma_stride(), 0, 0,
        width, info.height, width, height, libyuv::kRotate0,
        webrtc::ConvertVideoType(info.videoType));
    if (failed) return -1;
    // A capture larger than asked is scaled down to fit, keeping its shape,
    // at even sizes as encoders want.
    double scale = std::min({1.0, (double)max_width_ / width, (double)max_height_ / height});
    int fit_width = std::max(2, (int)(width * scale) & ~1);
    int fit_height = std::max(2, (int)(height * scale) & ~1);
    bool jpeg = info.videoType == webrtc::VideoType::kMJPEG;
    backcord_picture picture;
    if (fit_width == width && fit_height == height) {
      if (jpeg) converted_.limit_range();
      picture = converted_.picture();
    } else {
      scaled_.resize(fit_width, fit_height);
      libyuv::I420Scale(converted_.y(), converted_.width, converted_.u(),
                        converted_.chroma_stride(), converted_.v(),
                        converted_.chroma_stride(), width, height, scaled_.y(),
                        scaled_.width, scaled_.u(), scaled_.chroma_stride(), scaled_.v(),
                        scaled_.chroma_stride(), fit_width, fit_height,
                        libyuv::kFilterBox);
      // After scaling, over fewer pixels.
      if (jpeg) scaled_.limit_range();
      picture = scaled_.picture();
    }
    frame_(user_, &picture);
    return 0;
  }

 private:
  int max_width_;
  int max_height_;
  backcord_camera_frame frame_;
  void *user_;
  Planes converted_;
  Planes scaled_;
};

/* The device's mode to film `wanted` at: the smallest as tall as asked at
 * the frame rate asked, scaled down to it, so what goes out has the lines
 * chosen rather than the nearest mode's fewer; a raw format over MJPEG at
 * the same size. A device that can't film so tall gives its tallest, at the
 * rate if it can. One that lists no modes takes what was asked. */
webrtc::VideoCaptureCapability choose(webrtc::VideoCaptureModule::DeviceInfo &info,
                                      const char *id,
                                      const webrtc::VideoCaptureCapability &wanted) {
  int32_t count = info.NumberOfCapabilities(id);
  std::vector<webrtc::VideoCaptureCapability> modes;
  for (int32_t index = 0; index < count; index++) {
    webrtc::VideoCaptureCapability mode;
    if (info.GetCapability(id, index, mode) == 0 && mode.width > 0 && mode.height > 0) {
      modes.push_back(mode);
    }
  }
  if (modes.empty()) return wanted;
  auto fast = [&](const webrtc::VideoCaptureCapability &mode) {
    return mode.maxFPS <= 0 || mode.maxFPS >= wanted.maxFPS;
  };
  auto tall = [&](const webrtc::VideoCaptureCapability &mode) {
    return mode.height >= wanted.height;
  };
  auto pixels = [](const webrtc::VideoCaptureCapability &mode) {
    return (int64_t)mode.width * mode.height;
  };
  auto raw = [](const webrtc::VideoCaptureCapability &mode) {
    return mode.videoType != webrtc::VideoType::kMJPEG;
  };
  // The best of each kind: smaller, then raw, then faster.
  auto smaller = [&](const webrtc::VideoCaptureCapability &a,
                     const webrtc::VideoCaptureCapability &b) {
    if (pixels(a) != pixels(b)) return pixels(a) < pixels(b);
    if (raw(a) != raw(b)) return raw(a);
    return a.maxFPS > b.maxFPS;
  };
  const webrtc::VideoCaptureCapability *best = nullptr;
  for (const auto &mode : modes) {
    if (tall(mode) && fast(mode) && (!best || smaller(mode, *best))) best = &mode;
  }
  if (best) return *best;
  for (const auto &mode : modes) {
    if (tall(mode) && (!best || mode.maxFPS > best->maxFPS ||
                       (mode.maxFPS == best->maxFPS && smaller(mode, *best)))) {
      best = &mode;
    }
  }
  if (best) return *best;
  // Not so tall: its tallest, at the rate where it can.
  for (const auto &mode : modes) {
    if (!best || fast(mode) > fast(*best) ||
        (fast(mode) == fast(*best) &&
         (pixels(mode) > pixels(*best) || (pixels(mode) == pixels(*best) && raw(mode) && !raw(*best))))) {
      best = &mode;
    }
  }
  return *best;
}

}  // namespace

struct backcord_camera {
  webrtc::scoped_refptr<webrtc::VideoCaptureModule> module;
  std::unique_ptr<Sink> sink;
};

extern "C" {

typedef void (*backcord_camera_each)(void *user, const char *name, const char *id);

/* Calls `each` with the name and the id that opens it of each camera on this
 * system, from one look at the devices. */
void backcord_camera_list(backcord_camera_each each, void *user) {
  std::unique_ptr<webrtc::VideoCaptureModule::DeviceInfo> info(
      webrtc::VideoCaptureFactory::CreateDeviceInfo());
  if (!info) return;
  uint32_t count = info->NumberOfDevices();
  for (uint32_t index = 0; index < count; index++) {
    char name[256] = {0};
    char id[1024] = {0};
    if (info->GetDeviceName(index, name, sizeof name, id, sizeof id) == 0) {
      each(user, name, id);
    }
  }
}

/* Opens the camera with `id` and calls `frame` with each picture, at most
 * `width` by `height` and `fps` a second, until backcord_camera_close. */
struct backcord_camera *backcord_camera_open(const char *id, int width, int height, int fps,
                                     backcord_camera_frame frame, void *user) {
  std::unique_ptr<webrtc::VideoCaptureModule::DeviceInfo> info(
      webrtc::VideoCaptureFactory::CreateDeviceInfo());
  if (!info) return nullptr;
  webrtc::VideoCaptureCapability wanted;
  wanted.width = width;
  wanted.height = height;
  wanted.maxFPS = fps;
  webrtc::VideoCaptureCapability chosen = choose(*info, id, wanted);
  chosen.maxFPS = std::min(chosen.maxFPS > 0 ? chosen.maxFPS : fps, fps);

  auto camera = std::make_unique<backcord_camera>();
  camera->module = webrtc::VideoCaptureFactory::Create(id);
  if (!camera->module) return nullptr;
  camera->sink = std::make_unique<Sink>(width, height, frame, user);
  camera->module->RegisterCaptureDataCallback(camera->sink.get());
  if (camera->module->StartCapture(chosen) != 0) {
    camera->module->DeRegisterCaptureDataCallback();
    return nullptr;
  }
  return camera.release();
}

/* Stops capturing: no frame arrives once this returns. */
void backcord_camera_close(struct backcord_camera *camera) {
  if (!camera) return;
  camera->module->StopCapture();
  camera->module->DeRegisterCaptureDataCallback();
  delete camera;
}

}  // extern "C"
