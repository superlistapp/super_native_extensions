#include "include/super_native_extensions/super_native_extensions_plugin.h"

#include <flutter_linux/flutter_linux.h>
#include <gtk/gtk.h>
#include <sys/utsname.h>

#include <cstring>
#include <dlfcn.h>

#define SUPER_NATIVE_EXTENSIONS_PLUGIN(obj)                                    \
  (G_TYPE_CHECK_INSTANCE_CAST((obj),                                           \
                              super_native_extensions_plugin_get_type(),       \
                              SuperNativeExtensionsPlugin))

struct _SuperNativeExtensionsPlugin {
  GObject parent_instance;
};

G_DEFINE_TYPE(SuperNativeExtensionsPlugin, super_native_extensions_plugin,
              g_object_get_type())

static void super_native_extensions_plugin_dispose(GObject *object) {
  G_OBJECT_CLASS(super_native_extensions_plugin_parent_class)->dispose(object);
}

static void super_native_extensions_plugin_class_init(
    SuperNativeExtensionsPluginClass *klass) {
  G_OBJECT_CLASS(klass)->dispose = super_native_extensions_plugin_dispose;
}

extern "C" {
typedef void (*SNEInitFunction)(void);
}

static void
super_native_extensions_plugin_init(SuperNativeExtensionsPlugin *self) {
  static bool initialized = false;
  if (!initialized) {
    void *handle =
        dlopen("libsuper_native_extensions_native.so", RTLD_NOW | RTLD_LOCAL);
    if (handle == nullptr) {
      g_warning("Failed to load libsuper_native_extensions_native.so: %s",
                dlerror());
      return;
    }

    dlerror();
    auto init = reinterpret_cast<SNEInitFunction>(
        dlsym(handle, "super_native_extensions_init"));
    const char *error = dlerror();
    if (error != nullptr) {
      g_warning("Failed to resolve super_native_extensions_init: %s", error);
      return;
    }

    init();
    initialized = true;
  }
}

void super_native_extensions_plugin_register_with_registrar(
    FlPluginRegistrar *registrar) {
  SuperNativeExtensionsPlugin *plugin = SUPER_NATIVE_EXTENSIONS_PLUGIN(
      g_object_new(super_native_extensions_plugin_get_type(), nullptr));

  g_object_unref(plugin);
}
