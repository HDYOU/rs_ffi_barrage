#ifndef UTILS_H_
#define UTILS_H_

#include <flutter_windows.h>
#include <io.h>
#include <stdio.h>
#include <windows.h>

inline void CreateAndAttachConsole() {
  if (::AllocConsole()) {
    freopen("CONOUT$", "w", stdout);
    freopen("CONOUT$", "w", stderr);
  }
}

inline std::wstring Utf8ToWide(const std::string& utf8) {
  int wide_char_size = MultiByteToWideChar(CP_UTF8, 0, utf8.c_str(), -1, nullptr, 0);
  std::wstring wide_str(wide_char_size, 0);
  MultiByteToWideChar(CP_UTF8, 0, utf8.c_str(), -1, wide_str.data(), wide_char_size);
  return wide_str;
}

inline std::string WideToUtf8(const std::wstring& wide) {
  int utf8_size = WideCharToMultiByte(CP_UTF8, 0, wide.c_str(), -1, nullptr, 0, nullptr, nullptr);
  std::string utf8_str(utf8_size, 0);
  WideCharToMultiByte(CP_UTF8, 0, wide.c_str(), -1, utf8_str.data(), utf8_size, nullptr, nullptr);
  return utf8_str;
}

#endif  // UTILS_H_