#include "win32_window.h"

#include <dwmapi.h>
#include <flutter_windows.h>

#include "resource.h"

namespace {

constexpr const wchar_t kWindowClassName[] = L"FLUTTER_RUNNER_WIN32_WINDOW";

WindowProcDelegate g_window_proc_delegate = nullptr;

LRESULT CALLBACK WindowProc(HWND hwnd, UINT message, WPARAM wparam,
                            LPARAM lparam) {
  if (g_window_proc_delegate) {
    return g_window_proc_delegate(hwnd, message, wparam, lparam);
  }

  return DefWindowProc(hwnd, message, wparam, lparam);
}

}  // namespace

Win32Window::Win32Window() {}
Win32Window::~Win32Window() {}

bool Win32Window::CreateAndShow(const std::wstring& title,
                                const Point& origin,
                                const Size& size) {
  Destroy();

  const wchar_t* window_class =
      WindowClassRegistrar::GetInstance()->GetWindowClass();

  HWND hwnd = CreateWindowEx(0, window_class, title.c_str(),
                             WS_OVERLAPPEDWINDOW | WS_VISIBLE, origin.x,
                             origin.y, size.width, size.height, nullptr,
                             nullptr, GetModuleHandle(nullptr), this);

  return hwnd != nullptr;
}

void Win32Window::Destroy() {
  if (window_handle_) {
    DestroyWindow(window_handle_);
    window_handle_ = nullptr;
  }
}

Win32Window::WindowClassRegistrar* Win32Window::WindowClassRegistrar::GetInstance() {
  static WindowClassRegistrar* instance = new WindowClassRegistrar();
  return instance;
}

const wchar_t* Win32Window::WindowClassRegistrar::GetWindowClass() {
  if (!class_registered_) {
    WNDCLASS window_class = {};
    window_class.hCursor = LoadCursor(nullptr, IDC_ARROW);
    window_class.lpszClassName = kWindowClassName;
    window_class.style = CS_HREDRAW | CS_VREDRAW;
    window_class.cbClsExtra = 0;
    window_class.lpfnWndProc = WindowProc;
    RegisterClass(&window_class);
    class_registered_ = true;
  }
  return kWindowClassName;
}

void Win32Window::SetChildContent(HWND content) {
  SetParent(content, window_handle_);
  RECT frame = GetClientArea();

  MoveWindow(content, frame.left, frame.top, frame.right - frame.left,
             frame.bottom - frame.top, true);

  SetFocus(content);
}

RECT Win32Window::GetClientArea() {
  RECT frame;
  GetClientRect(window_handle_, &frame);
  return frame;
}

HWND Win32Window::GetHandle() {
  return window_handle_;
}

void Win32Window::SetWindowProcDelegate(WindowProcDelegate delegate) {
  g_window_proc_delegate = delegate;
}

void Win32Window::OnCreate() {}

void Win32Window::OnDestroy() {}