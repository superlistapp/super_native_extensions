#ifndef FLUTTER_PLUGIN_SUPER_NATIVE_EXTENSIONS_PLUGIN_H_
#define FLUTTER_PLUGIN_SUPER_NATIVE_EXTENSIONS_PLUGIN_H_

#include <flutter/method_channel.h>
#include <flutter/plugin_registrar_windows.h>

#include <memory>

namespace super_native_extensions {

class SuperNativeExtensionsPlugin : public flutter::Plugin {
public:
  static void RegisterWithRegistrar(flutter::PluginRegistrarWindows *registrar);

  SuperNativeExtensionsPlugin();

  virtual ~SuperNativeExtensionsPlugin();

  // Disallow copy and assign.
  SuperNativeExtensionsPlugin(const SuperNativeExtensionsPlugin &) = delete;
  SuperNativeExtensionsPlugin &
  operator=(const SuperNativeExtensionsPlugin &) = delete;

private:
};

} // namespace super_native_extensions

#endif // FLUTTER_PLUGIN_SUPER_NATIVE_EXTENSIONS_PLUGIN_H_
