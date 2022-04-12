#include "include/super_data_transfer/super_data_transfer_plugin.h"

#include <flutter_linux/flutter_linux.h>
#include <gtk/gtk.h>
#include <sys/utsname.h>

#include <cstring>

#define SUPER_DATA_TRANSFER_PLUGIN(obj) \
  (G_TYPE_CHECK_INSTANCE_CAST((obj), super_data_transfer_plugin_get_type(), \
                              SuperDataTransferPlugin))

struct _SuperDataTransferPlugin {
  GObject parent_instance;
};

G_DEFINE_TYPE(SuperDataTransferPlugin, super_data_transfer_plugin, g_object_get_type())

static void super_data_transfer_plugin_dispose(GObject* object) {
  G_OBJECT_CLASS(super_data_transfer_plugin_parent_class)->dispose(object);
}

static void super_data_transfer_plugin_class_init(SuperDataTransferPluginClass* klass) {
  G_OBJECT_CLASS(klass)->dispose = super_data_transfer_plugin_dispose;
}

extern "C" {
  extern void super_data_transfer_init(void);
}

static void super_data_transfer_plugin_init(SuperDataTransferPlugin* self) {
  super_data_transfer_init();
}

void super_data_transfer_plugin_register_with_registrar(FlPluginRegistrar* registrar) {
  SuperDataTransferPlugin* plugin = SUPER_DATA_TRANSFER_PLUGIN(
      g_object_new(super_data_transfer_plugin_get_type(), nullptr));
  g_object_unref(plugin);
}
