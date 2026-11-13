import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Shell from 'gi://Shell';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const IFACE = `<node><interface name="org.tinyhand.Test">
  <method name="Move"><arg type="d" direction="in"/><arg type="d" direction="in"/></method>
  <method name="Button"><arg type="b" direction="in"/></method>
  <method name="Key"><arg type="u" direction="in"/><arg type="b" direction="in"/></method>
  <method name="Shot"><arg type="s" direction="in"/></method>
  <method name="Windows"><arg type="s" direction="out"/></method>
  <method name="Cursor"><arg type="s" direction="out"/></method>
  <method name="Overview"><arg type="b" direction="in"/></method>
  <method name="Workspace"><arg type="i" direction="in"/></method>
</interface></node>`;

export default class TestDriver extends Extension {
    enable() {
        const seat = Clutter.get_default_backend().get_default_seat();
        this._pointer = seat.create_virtual_device(Clutter.InputDeviceType.POINTER_DEVICE);
        this._keyboard = seat.create_virtual_device(Clutter.InputDeviceType.KEYBOARD_DEVICE);
        this._dbus = Gio.DBusExportedObject.wrapJSObject(IFACE, this);
        this._dbus.export(Gio.DBus.session, '/org/tinyhand/Test');
        this._name = Gio.bus_own_name(Gio.BusType.SESSION, 'org.tinyhand.Test', 0, null, null, null);
    }

    disable() {
        this._dbus.unexport();
        Gio.bus_unown_name(this._name);
    }

    Move(x, y) {
        this._pointer.notify_absolute_motion(GLib.get_monotonic_time(), x, y);
    }

    Button(press) {
        this._pointer.notify_button(GLib.get_monotonic_time(), Clutter.BUTTON_PRIMARY,
            press ? Clutter.ButtonState.PRESSED : Clutter.ButtonState.RELEASED);
    }

    Key(keysym, press) {
        this._keyboard.notify_keyval(GLib.get_monotonic_time(), keysym,
            press ? Clutter.KeyState.PRESSED : Clutter.KeyState.RELEASED);
    }

    Shot(path) {
        const shooter = new Shell.Screenshot();
        const stream = Gio.File.new_for_path(path).replace(null, false, Gio.FileCreateFlags.NONE, null);
        shooter.screenshot(false, stream, () => stream.close(null));
    }

    Cursor() {
        const sprite = global.backend.get_cursor_tracker().get_sprite();
        return sprite ? `${sprite.get_width()}x${sprite.get_height()}` : 'none';
    }

    Windows() {
        const manager = global.workspace_manager;
        return JSON.stringify(global.get_window_actors().map(actor => {
            const w = actor.meta_window;
            const r = w.get_frame_rect();
            return {
                title: w.get_title(), wmClass: w.get_wm_class(), appId: w.get_gtk_application_id?.() ?? null,
                above: w.is_above(), sticky: w.is_on_all_workspaces(), monitor: w.get_monitor(),
                focused: w.has_focus(), rect: [r.x, r.y, r.width, r.height],
                layer: w.get_layer?.() ?? null, workspace: manager.get_active_workspace_index(),
                top: global.display.sort_windows_by_stacking(global.display.list_all_windows()).at(-1) === w,
            };
        }));
    }

    Overview(show) {
        if (show)
            Main.overview.show();
        else
            Main.overview.hide();
    }

    Workspace(index) {
        const manager = global.workspace_manager;
        while (manager.get_n_workspaces() <= index)
            manager.append_new_workspace(false, global.get_current_time());
        manager.get_workspace_by_index(index).activate(global.get_current_time());
    }
}
