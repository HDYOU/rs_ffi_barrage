#ifndef WIN32_WINDOW_H_
#define WIN32_WINDOW_H_

#include <windows.h>

#include <functional>
#include <memory>
#include <string>

class Win32Window {
 public:
  struct Point {
    unsigned int x;
    unsigned int y;
    Point(unsigned int x, unsigned int y) : x(x), y(y) {}
  };

  struct Size {
    unsigned int width;
    unsigned int height;
    Size(unsigned int width, unsigned int height)
        : width(width), height(height) {}
  };

  Win32Window();
  virtual ~Win32Window();

  bool CreateAndShow(const std::wstring& title, const Point& origin,
                     const Size& size);
  void Destroy();
  void SetChildContent(HWND content);
  RECT GetClientArea();
  HWND GetHandle();

  using WindowProcDelegate =
      std::function<LRESULT(HWND, UINT, WPARAM, LPARAM)>;
  void SetWindowProcDelegate(WindowProcDelegate delegate);

 protected:
  virtual void OnCreate();
  virtual void OnDestroy();

 private:
  class WindowClassRegistrar {
   public:
    static WindowClassRegistrar* GetInstance();
    const wchar_t* GetWindowClass();

   private:
    WindowClassRegistrar() = default;
    bool class_registered_ = false;
  };

  HWND window_handle_ = nullptr;
};

#endif  // WIN32_WINDOW_H_