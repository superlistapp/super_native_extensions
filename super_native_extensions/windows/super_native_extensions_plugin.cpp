#include "super_native_extensions_plugin.h"

// This must be included before many other Windows headers.
#include <windows.h>

#include <flutter/method_channel.h>
#include <flutter/plugin_registrar_windows.h>
#include <flutter/standard_method_codec.h>

#include <memory>
#include <sstream>

extern "C" {
extern void super_native_extensions_init(void);
}

namespace super_native_extensions {

// static
void SuperNativeExtensionsPlugin::RegisterWithRegistrar(
    flutter::PluginRegistrarWindows *registrar) {

  super_native_extensions_init();

  auto channel =
      std::make_unique<flutter::MethodChannel<flutter::EncodableValue>>(
          registrar->messenger(), "super_native_extensions",
          &flutter::StandardMethodCodec::GetInstance());

  auto plugin = std::make_unique<SuperNativeExtensionsPlugin>(
      registrar->GetView()->GetNativeWindow());

  channel->SetMethodCallHandler(
      [plugin_pointer = plugin.get()](const auto &call, auto result) {
        plugin_pointer->HandleMethodCall(call, std::move(result));
      });

  registrar->AddPlugin(std::move(plugin));
}

SuperNativeExtensionsPlugin::SuperNativeExtensionsPlugin(HWND hwnd)
    : _hwnd(hwnd) {}

SuperNativeExtensionsPlugin::~SuperNativeExtensionsPlugin() {}

void SuperNativeExtensionsPlugin::HandleMethodCall(
    const flutter::MethodCall<flutter::EncodableValue> &method_call,
    std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result) {
  if (method_call.method_name().compare("getFlutterView") == 0) {
    result->Success(flutter::EncodableValue(reinterpret_cast<int64_t>(_hwnd)));
  } else {
    result->NotImplemented();
  }
}

} // namespace super_native_extensions
