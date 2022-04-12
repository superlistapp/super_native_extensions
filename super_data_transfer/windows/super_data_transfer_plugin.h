#ifndef FLUTTER_PLUGIN_SUPER_DATA_TRANSFER_PLUGIN_H_
#define FLUTTER_PLUGIN_SUPER_DATA_TRANSFER_PLUGIN_H_

#include <flutter/method_channel.h>
#include <flutter/plugin_registrar_windows.h>

#include <memory>

namespace super_data_transfer {

class SuperDataTransferPlugin : public flutter::Plugin {
 public:
  static void RegisterWithRegistrar(flutter::PluginRegistrarWindows *registrar);

  SuperDataTransferPlugin();

  virtual ~SuperDataTransferPlugin();

  // Disallow copy and assign.
  SuperDataTransferPlugin(const SuperDataTransferPlugin&) = delete;
  SuperDataTransferPlugin& operator=(const SuperDataTransferPlugin&) = delete;

 private:
  // Called when a method is called on this plugin's channel from Dart.
  void HandleMethodCall(
      const flutter::MethodCall<flutter::EncodableValue> &method_call,
      std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result);
};

}  // namespace super_data_transfer

#endif  // FLUTTER_PLUGIN_SUPER_DATA_TRANSFER_PLUGIN_H_
