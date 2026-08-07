#include "super_native_extensions_plugin.h"

// This must be included before many other Windows headers.
#include <windows.h>

#include <flutter/method_channel.h>
#include <flutter/plugin_registrar_windows.h>
#include <flutter/standard_method_codec.h>

#include <memory>
#include <sstream>

namespace super_native_extensions {

namespace {

using SNEInitFunction = void (*)();

void LogLastError(const char *message) {
  std::ostringstream stream;
  stream << message << " Error: " << GetLastError() << "\n";
  OutputDebugStringA(stream.str().c_str());
}

void InitializeRustLibrary() {
  static bool initialized = false;
  if (initialized) {
    return;
  }

  HMODULE module = LoadLibraryW(L"super_native_extensions_native.dll");
  if (module == nullptr) {
    LogLastError("Failed to load super_native_extensions_native.dll.");
    return;
  }

  auto init = reinterpret_cast<SNEInitFunction>(
      GetProcAddress(module, "super_native_extensions_init"));
  if (init == nullptr) {
    LogLastError("Failed to resolve super_native_extensions_init.");
    return;
  }

  init();
  initialized = true;
}

} // namespace

// static
void SuperNativeExtensionsPlugin::RegisterWithRegistrar(
    flutter::PluginRegistrarWindows *registrar) {

  InitializeRustLibrary();

  auto plugin = std::make_unique<SuperNativeExtensionsPlugin>();

  registrar->AddPlugin(std::move(plugin));
}

SuperNativeExtensionsPlugin::SuperNativeExtensionsPlugin() {}

SuperNativeExtensionsPlugin::~SuperNativeExtensionsPlugin() {}

} // namespace super_native_extensions
