#include "include/super_data_transfer/super_data_transfer_plugin_c_api.h"

#include <flutter/plugin_registrar_windows.h>

#include "super_data_transfer_plugin.h"

void SuperDataTransferPluginCApiRegisterWithRegistrar(
    FlutterDesktopPluginRegistrarRef registrar) {
  super_data_transfer::SuperDataTransferPlugin::RegisterWithRegistrar(
      flutter::PluginRegistrarManager::GetInstance()
          ->GetRegistrar<flutter::PluginRegistrarWindows>(registrar));
}
