/*
 * Copyright 2025 LiveKit, Inc.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#include "livekit/desktop_capturer.h"

#include "modules/desktop_capture/desktop_capture_options.h"

#ifdef _WIN64
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>
#include <dwmapi.h>

#include <cwchar>
#include <string>
#include <vector>
#endif /* _WIN64 */

using SourceList = webrtc::DesktopCapturer::SourceList;

#ifdef _WIN64
namespace {

// BACKCORD: libwebrtc leaves out windows slower than 50 ms to answer, as a
// busy game's often is, windows another window owns and tool windows,
// though it captures them as well as any. These are the windows a person
// sees on this desktop, BACKCORD's own aside.
struct WindowListing {
  DWORD self;
  SourceList found;
};

std::string Utf8(const std::wstring& text) {
  int size = WideCharToMultiByte(CP_UTF8, 0, text.data(),
                                 static_cast<int>(text.size()), nullptr, 0,
                                 nullptr, nullptr);
  std::string utf8(size > 0 ? size : 0, '\0');
  if (size > 0) {
    WideCharToMultiByte(CP_UTF8, 0, text.data(), static_cast<int>(text.size()),
                        utf8.data(), size, nullptr, nullptr);
  }
  return utf8;
}

BOOL CALLBACK ListWindow(HWND hwnd, LPARAM param) {
  auto* listing = reinterpret_cast<WindowListing*>(param);
  if (!IsWindowVisible(hwnd)) {
    return TRUE;
  }
  DWORD process = 0;
  GetWindowThreadProcessId(hwnd, &process);
  if (process == listing->self) {
    return TRUE;
  }
  // On another virtual desktop, or a store app the system suspended.
  DWORD cloaked = 0;
  if (SUCCEEDED(DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &cloaked,
                                      sizeof(cloaked))) &&
      cloaked) {
    return TRUE;
  }
  LONG_PTR style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
  if ((style & WS_EX_TOOLWINDOW) && !(style & WS_EX_APPWINDOW)) {
    return TRUE;
  }
  // Too small to be anything to share, minimized windows aside.
  RECT rect{};
  if (!IsIconic(hwnd) && GetWindowRect(hwnd, &rect) &&
      (rect.right - rect.left < 64 || rect.bottom - rect.top < 64)) {
    return TRUE;
  }
  wchar_t name[64] = {};
  GetClassNameW(hwnd, name, 64);
  for (const wchar_t* shell :
       {L"Progman", L"WorkerW", L"Button", L"Shell_TrayWnd",
        L"Shell_SecondaryTrayWnd", L"Windows.UI.Core.CoreWindow"}) {
    if (wcscmp(name, shell) == 0) {
      return TRUE;
    }
  }
  // Another process's title is read without asking it, so a window that
  // doesn't answer doesn't hold the list up.
  int length = GetWindowTextLengthW(hwnd);
  if (length <= 0) {
    return TRUE;
  }
  std::wstring title(length + 1, L'\0');
  title.resize(GetWindowTextW(hwnd, title.data(), length + 1));
  if (title.empty()) {
    return TRUE;
  }
  webrtc::DesktopCapturer::Source source;
  source.id = reinterpret_cast<webrtc::DesktopCapturer::SourceId>(hwnd);
  source.title = Utf8(title);
  listing->found.push_back(source);
  return TRUE;
}

// The windows, top first, with any libwebrtc listed and these didn't after.
SourceList AllWindows(const SourceList& listed) {
  WindowListing listing{GetCurrentProcessId(), {}};
  EnumWindows(ListWindow, reinterpret_cast<LPARAM>(&listing));
  for (const auto& source : listed) {
    bool known = false;
    for (const auto& found : listing.found) {
      known = known || found.id == source.id;
    }
    if (!known) {
      listing.found.push_back(source);
    }
  }
  return listing.found;
}

}  // namespace
#endif /* _WIN64 */

namespace livekit_ffi {

std::unique_ptr<DesktopCapturer> new_desktop_capturer(
    DesktopCapturerOptions options) {
  webrtc::DesktopCaptureOptions webrtc_options =
      webrtc::DesktopCaptureOptions::CreateDefault();
#if defined(WEBRTC_MAC) && !defined(WEBRTC_IOS)
  webrtc_options.set_allow_sck_capturer(true);
  webrtc_options.set_allow_sck_system_picker(options.allow_sck_system_picker);
#endif /* defined(WEBRTC_MAC) && !defined(WEBRTC_IOS) */
#ifdef _WIN64
  switch (options.source_type) {
    case SourceType::Screen:
      webrtc_options.set_allow_wgc_screen_capturer(true);
      break;
    case SourceType::Window:
      webrtc_options.set_allow_wgc_window_capturer(true);
      // https://github.com/webrtc-sdk/webrtc/blob/m137_release/modules/desktop_capture/desktop_capture_options.h#L133-L142
      webrtc_options.set_enumerate_current_process_windows(false);
      break;
    default:
      break;
  }
  webrtc_options.set_allow_directx_capturer(true);
#endif /* _WIN64 */
#ifdef WEBRTC_USE_PIPEWIRE
  webrtc_options.set_allow_pipewire(true);
#endif /* WEBRTC_USE_PIPEWIRE */

  // prefer_cursor_embedded indicate that the capturer should try to include the
  // cursor in the frame
  webrtc_options.set_prefer_cursor_embedded(options.include_cursor);

  std::unique_ptr<webrtc::DesktopCapturer> capturer = nullptr;
  switch (options.source_type) {
    case SourceType::Window:
      capturer = webrtc::DesktopCapturer::CreateWindowCapturer(webrtc_options);
      break;
    case SourceType::Screen:
      capturer = webrtc::DesktopCapturer::CreateScreenCapturer(webrtc_options);
      break;
    case SourceType::Generic:
      capturer = webrtc::DesktopCapturer::CreateGenericCapturer(webrtc_options);
      break;
    default:
      return nullptr;
  }

  if (!capturer) {
    return nullptr;
  }
  auto made = std::make_unique<DesktopCapturer>(std::move(capturer));
  made->lists_windows = options.source_type == SourceType::Window;
  return made;
}

void DesktopCapturer::start(
    rust::Box<DesktopCapturerCallbackWrapper> callback) {
  this->callback = std::move(callback);
  capturer->Start(this);
}

void DesktopCapturer::OnCaptureResult(
    webrtc::DesktopCapturer::Result result,
    std::unique_ptr<webrtc::DesktopFrame> frame) {
  CaptureResult ret_result = CaptureResult::ErrorPermanent;
  switch (result) {
    case webrtc::DesktopCapturer::Result::SUCCESS:
      ret_result = CaptureResult::Success;
      break;
    case webrtc::DesktopCapturer::Result::ERROR_PERMANENT:
      ret_result = CaptureResult::ErrorPermanent;
      break;
    case webrtc::DesktopCapturer::Result::ERROR_TEMPORARY:
      ret_result = CaptureResult::ErrorTemporary;
      break;
    default:
      break;
  }
  if (callback) {
    (*callback)->on_capture_result(
        ret_result, std::make_unique<DesktopFrame>(std::move(frame)));
  }
}

rust::Vec<Source> DesktopCapturer::get_source_list() const {
  SourceList list{};
  bool res = capturer->GetSourceList(&list);
#ifdef _WIN64
  if (lists_windows) {
    list = AllWindows(res ? list : SourceList{});
    res = true;
  }
#endif /* _WIN64 */
  rust::Vec<Source> source_list{};
  if (res) {
    for (auto& source : list) {
      source_list.push_back(Source{static_cast<uint64_t>(source.id),
                                   source.title, source.display_id});
    }
  }
  return source_list;
}
}  // namespace livekit_ffi