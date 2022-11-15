//
//  Generated file. Do not edit.
//

// clang-format off

#include "generated_plugin_registrant.h"

#include <ironbird_engine_context/ironbird_engine_context_plugin.h>
#include <super_native_extensions/super_native_extensions_plugin.h>

void fl_register_plugins(FlPluginRegistry* registry) {
  g_autoptr(FlPluginRegistrar) ironbird_engine_context_registrar =
      fl_plugin_registry_get_registrar_for_plugin(registry, "IronbirdEngineContextPlugin");
  ironbird_engine_context_plugin_register_with_registrar(ironbird_engine_context_registrar);
  g_autoptr(FlPluginRegistrar) super_native_extensions_registrar =
      fl_plugin_registry_get_registrar_for_plugin(registry, "SuperNativeExtensionsPlugin");
  super_native_extensions_plugin_register_with_registrar(super_native_extensions_registrar);
}
