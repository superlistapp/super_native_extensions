//
//  Generated file. Do not edit.
//

// clang-format off

#include "generated_plugin_registrant.h"

#include <super_data_transfer/super_data_transfer_plugin.h>

void fl_register_plugins(FlPluginRegistry* registry) {
  g_autoptr(FlPluginRegistrar) super_data_transfer_registrar =
      fl_plugin_registry_get_registrar_for_plugin(registry, "SuperDataTransferPlugin");
  super_data_transfer_plugin_register_with_registrar(super_data_transfer_registrar);
}
