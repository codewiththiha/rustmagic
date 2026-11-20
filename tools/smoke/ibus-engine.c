/*
 * Drives the RustMagic IBus engine the way the IBus daemon drives it.
 *
 * The daemon asks a service for an engine through its factory, then sends key
 * events and reads what comes back. This program does exactly that with a real
 * D-Bus connection, and decodes the committed text with libibus itself — so a
 * payload libibus cannot read fails here rather than in a user's session.
 *
 * Build:  cc ibus-engine.c $(pkg-config --cflags --libs ibus-1.0) -o ibus-smoke
 * Run:    DBUS_SESSION_BUS_ADDRESS=... ./ibus-smoke <bus-name> <engine-name>
 */

#include <ibus.h>
#include <stdio.h>
#include <string.h>

static const char *FACTORY_PATH = "/org/freedesktop/IBus/Factory";
static const char *FACTORY_IFACE = "org.freedesktop.IBus.Factory";
static const char *ENGINE_IFACE = "org.freedesktop.IBus.Engine";

static GMainLoop *loop = NULL;
static char committed[1024] = {0};
static int commits = 0;
static char preedit[1024] = {0};
static int preedits = 0;

static void on_signal(GDBusConnection *connection, const char *sender,
                      const char *object_path, const char *interface,
                      const char *signal, GVariant *parameters,
                      gpointer data) {
    (void)connection; (void)sender; (void)object_path; (void)interface;
    (void)data;
    GVariant *value = g_variant_get_child_value(parameters, 0);
    IBusSerializable *serializable = ibus_serializable_deserialize(value);
    if (serializable == NULL) {
        fprintf(stderr, "libibus could not decode the payload of %s\n", signal);
        g_variant_unref(value);
        return;
    }

    if (g_strcmp0(signal, "CommitText") == 0 && IBUS_IS_TEXT(serializable)) {
        const gchar *text = ibus_text_get_text(IBUS_TEXT(serializable));
        g_strlcpy(committed, text ? text : "", sizeof(committed));
        commits++;
    } else if (g_strcmp0(signal, "UpdatePreeditText") == 0 && IBUS_IS_TEXT(serializable)) {
        const gchar *text = ibus_text_get_text(IBUS_TEXT(serializable));
        g_strlcpy(preedit, text ? text : "", sizeof(preedit));
        preedits++;
    }
    g_object_unref(serializable);
    g_variant_unref(value);
}

static gboolean call(GDBusConnection *bus, const char *dest, const char *path,
                     const char *iface, const char *method, GVariant *args,
                     GVariant **reply) {
    GError *error = NULL;
    GVariant *result = g_dbus_connection_call_sync(
        bus, dest, path, iface, method, args, NULL,
        G_DBUS_CALL_FLAGS_NONE, 5000, NULL, &error);
    if (result == NULL) {
        fprintf(stderr, "call %s failed: %s\n", method,
                error ? error->message : "unknown");
        if (error != NULL) g_error_free(error);
        return FALSE;
    }
    if (reply != NULL) *reply = result; else g_variant_unref(result);
    return TRUE;
}

static gboolean process_key(GDBusConnection *bus, const char *dest, const char *path,
                            guint keyval, guint keycode, guint state) {
    GVariant *reply = NULL;
    GVariant *args = g_variant_new("(uuu)", keyval, keycode, state);
    if (!call(bus, dest, path, ENGINE_IFACE, "ProcessKeyEvent", args, &reply))
        return FALSE;
    gboolean handled = FALSE;
    g_variant_get(reply, "(b)", &handled);
    g_variant_unref(reply);
    /* Let the queued signals arrive before the next step. */
    while (g_main_context_iteration(NULL, FALSE)) {}
    return handled;
}

int main(int argc, char **argv) {
    const char *bus_name = argc > 1 ? argv[1] : "org.freedesktop.IBus.RustMagic";
    const char *engine_name = argc > 2 ? argv[2] : "rustmagic";

    ibus_init();
    GError *error = NULL;
    GDBusConnection *bus = g_bus_get_sync(G_BUS_TYPE_SESSION, NULL, &error);
    if (bus == NULL) {
        fprintf(stderr, "no session bus: %s\n", error ? error->message : "unknown");
        return 1;
    }

    GVariant *reply = NULL;
    if (!call(bus, bus_name, FACTORY_PATH, FACTORY_IFACE, "CreateEngine",
              g_variant_new("(s)", engine_name), &reply)) {
        return 1;
    }
    const char *engine_path = NULL;
    g_variant_get(reply, "(&o)", &engine_path);
    char path[256];
    g_strlcpy(path, engine_path, sizeof(path));
    g_variant_unref(reply);
    printf("engine at %s\n", path);

    const char *signals[] = {"CommitText", "UpdatePreeditText"};
    for (guint index = 0; index < G_N_ELEMENTS(signals); index++) {
        g_dbus_connection_signal_subscribe(
            bus, bus_name, ENGINE_IFACE, signals[index], path, NULL,
            G_DBUS_SIGNAL_FLAGS_NONE, on_signal, NULL, NULL);
    }

    int failures = 0;

    /* "a" is the first key of the demo Greek layout: it should show as a preedit
     * and nothing is committed yet. */
    if (!process_key(bus, bus_name, path, 0x0061, 38, 0)) {
        fprintf(stderr, "FAIL the engine did not handle `a`\n");
        failures++;
    }
    if (g_strcmp0(preedit, "α") != 0) {
        fprintf(stderr, "FAIL preedit is `%s`, expected `α`\n", preedit);
        failures++;
    }
    if (commits != 0) {
        fprintf(stderr, "FAIL something was committed too early: `%s`\n", committed);
        failures++;
    }

    /* Space ends the syllable: it commits the composition and is handed back. */
    if (process_key(bus, bus_name, path, 0x0020, 65, 0)) {
        fprintf(stderr, "FAIL the engine swallowed the space\n");
        failures++;
    }
    if (g_strcmp0(committed, "α") != 0) {
        fprintf(stderr, "FAIL committed `%s`, expected `α`\n", committed);
        failures++;
    }
    if (commits != 1) {
        fprintf(stderr, "FAIL %d commits, expected one\n", commits);
        failures++;
    }

    /* A real Zawgyi keyboard if the library has one would be checked here; the
     * demo is enough to prove the plumbing, the payload and the policy. */
    printf("committed `%s` after %d commit(s) and %d preedit update(s)\n",
           committed, commits, preedits);

    g_object_unref(bus);
    if (loop != NULL) g_main_loop_unref(loop);
    if (failures != 0) {
        fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    printf("the engine speaks IBus\n");
    return 0;
}
