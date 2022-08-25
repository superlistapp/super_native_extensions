#include "include/super_native_extensions/super_native_extensions_plugin.h"

#include <flutter_linux/flutter_linux.h>
#include <gtk/gtk.h>
#include <sys/utsname.h>

#include <cstring>

#define SUPER_NATIVE_EXTENSIONS_PLUGIN(obj)                                    \
  (G_TYPE_CHECK_INSTANCE_CAST((obj),                                           \
                              super_native_extensions_plugin_get_type(),       \
                              SuperNativeExtensionsPlugin))

struct _SuperNativeExtensionsPlugin {
  GObject parent_instance;
  FlView *view;
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
extern void super_native_extensions_init(void);
}

// Called when a method call is received from Flutter.
static void super_native_extensions_plugin_handle_method_call(
    SuperNativeExtensionsPlugin *self, FlMethodCall *method_call) {
  const gchar *method = fl_method_call_get_name(method_call);
  g_autoptr(FlMethodResponse) response = nullptr;
  if (strcmp(method, "getFlutterView") == 0) {
    g_autoptr(FlValue) result =
        fl_value_new_int(reinterpret_cast<uint64_t>(self->view));
    response = FL_METHOD_RESPONSE(fl_method_success_response_new(result));
  } else {
    response = FL_METHOD_RESPONSE(fl_method_not_implemented_response_new());
  }
  g_autoptr(GError) error = nullptr;
  if (!fl_method_call_respond(method_call, response, &error))
    g_warning("Failed to send method call response: %s", error->message);
}

static void method_call_cb(FlMethodChannel *channel, FlMethodCall *method_call,
                           gpointer user_data) {
  SuperNativeExtensionsPlugin *plugin =
      SUPER_NATIVE_EXTENSIONS_PLUGIN(user_data);
  super_native_extensions_plugin_handle_method_call(plugin, method_call);
}

static void
super_native_extensions_plugin_init(SuperNativeExtensionsPlugin *self) {
  super_native_extensions_init();
}

void super_native_extensions_plugin_register_with_registrar(
    FlPluginRegistrar *registrar) {
  SuperNativeExtensionsPlugin *plugin = SUPER_NATIVE_EXTENSIONS_PLUGIN(
      g_object_new(super_native_extensions_plugin_get_type(), nullptr));

  plugin->view = fl_plugin_registrar_get_view(registrar);

  g_autoptr(FlStandardMethodCodec) codec = fl_standard_method_codec_new();
  FlMethodChannel *channel =
      fl_method_channel_new(fl_plugin_registrar_get_messenger(registrar),
                            "super_native_extensions", FL_METHOD_CODEC(codec));

  fl_method_channel_set_method_call_handler(
      channel, method_call_cb, g_object_ref(plugin), g_object_unref);

  g_object_unref(plugin);
}
