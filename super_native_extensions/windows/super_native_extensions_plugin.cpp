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

  static bool initialized = false;
  if (!initialized) {
    super_native_extensions_init();
    initialized = true;
  }

  auto plugin = std::make_unique<SuperNativeExtensionsPlugin>();

  registrar->AddPlugin(std::move(plugin));
}

SuperNativeExtensionsPlugin::SuperNativeExtensionsPlugin() {}

SuperNativeExtensionsPlugin::~SuperNativeExtensionsPlugin() {}

} // namespace super_native_extensions
