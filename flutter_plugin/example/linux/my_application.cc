#include "my_application.h"

#include <flutter_linux/flutter_linux.h>

G_DEFINE_TYPE(MyApplication, my_application, GTK_APPLICATION_TYPE)

static void my_application_init(MyApplication* self) {}

static void my_application_class_init(MyApplicationClass* klass) {}

static void my_application_activate(GApplication* application) {
  MyApplication* self = MY_APPLICATION(application);
  GtkWindow* window =
      GTK_WINDOW(gtk_application_window_new(GTK_APPLICATION(application)));

  gtk_window_set_title(window, "rs_ffi_barrage_example");
  gtk_window_set_default_size(window, 1280, 720);
  gtk_widget_show(GTK_WIDGET(window));

  g_autoptr(FlDartProject) project = fl_dart_project_new();
  fl_dart_project_set_dart_entrypoint_arguments(project, nullptr);

  FlView* view = fl_view_new(project);
  gtk_widget_show(GTK_WIDGET(view));
  gtk_container_add(GTK_CONTAINER(window), GTK_WIDGET(view));

  fl_view_register_plugins(FL_VIEW(view));
}

MyApplication* my_application_new() {
  return MY_APPLICATION(g_object_new(
      my_application_get_type(), "application-id", "com.example.rs-ffi-barrage-example",
      nullptr));
}