//
//  Generated file. Do not edit.
//

// clang-format off

#include "generated_plugin_registrant.h"

#include <flutter_engine_context/flutter_engine_context_plugin_c_api.h>
#include <super_native_extensions/super_native_extensions_plugin_c_api.h>

void RegisterPlugins(flutter::PluginRegistry* registry) {
  FlutterEngineContextPluginCApiRegisterWithRegistrar(
      registry->GetRegistrarForPlugin("FlutterEngineContextPluginCApi"));
  SuperNativeExtensionsPluginCApiRegisterWithRegistrar(
      registry->GetRegistrarForPlugin("SuperNativeExtensionsPluginCApi"));
}
