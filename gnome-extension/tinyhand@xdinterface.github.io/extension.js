import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import {PACKAGE_VERSION} from 'resource:///org/gnome/shell/misc/config.js';

const APP_ID = 'io.github.xdinterface.TinyHand';
const OBJECT_PATH = '/io/github/xdinterface/TinyHand';
const PREFIX = 'TinyHand Overlay';
const PASS = ' [pass]';
const SHELL_MAJOR = parseInt(PACKAGE_VERSION.split('.')[0]);

function isOverlay(win) {
    const title = win.get_title() ?? '';
    const app = win.get_gtk_application_id?.() ?? win.get_wm_class();
    return title.startsWith(PREFIX) && app === APP_ID;
}

const SHORTCUTS = {toggle: 'ctrl+alt+h', clear: 'ctrl+alt+x'};
const MODIFIERS = {ctrl: '<Control>', control: '<Control>', alt: '<Alt>', shift: '<Shift>', super: '<Super>'};

function activate(action, flags) {
    Gio.DBus.session.call(
        APP_ID, OBJECT_PATH, 'org.gtk.Actions', 'Activate',
        new GLib.Variant('(sava{sv})', [action, [], {}]),
        null, flags, -1, null, null);
}

function configPath() {
    return GLib.getenv('TINYHAND_CONFIG') ??
        GLib.build_filenamev([GLib.get_user_config_dir(), 'tinyhand', 'config.toml']);
}

function accelerator(text) {
    const parts = text.toLowerCase().split('+').map(part => part.trim());
    const key = parts.pop();
    if (!key || key === 'off' || parts.some(part => !(part in MODIFIERS)))
        return null;
    return parts.map(part => MODIFIERS[part]).join('') + key;
}

function readShortcuts() {
    let text = '';
    try {
        text = new TextDecoder().decode(GLib.file_get_contents(configPath())[1]);
    } catch {
    }
    const shortcuts = {};
    for (const [action, fallback] of Object.entries(SHORTCUTS)) {
        const match = text.match(new RegExp(`^\\s*${action}-shortcut\\s*=\\s*"([^"]*)"`, 'm'));
        shortcuts[action] = accelerator(match ? match[1] : fallback);
    }
    return shortcuts;
}

export default class TinyHandExtension extends Extension {
    enable() {
        this._watched = new Map();
        this._placed = new WeakSet();
        this._created = global.display.connect('window-created', (_display, win) => this._watch(win));
        this._overview = Main.overview.connect('showing', () => {
            if (global.get_window_actors().some(actor => isOverlay(actor.meta_window)))
                activate('off', Gio.DBusCallFlags.NO_AUTO_START);
        });
        this._grabs = new Map();
        this._activated = global.display.connect('accelerator-activated', (_display, id) => {
            if (this._grabs.has(id))
                activate(this._grabs.get(id), Gio.DBusCallFlags.NONE);
        });
        this._config = Gio.File.new_for_path(configPath()).monitor_file(Gio.FileMonitorFlags.NONE, null);
        this._config.connect('changed', () => this._grab());
        this._grab();
        for (const actor of global.get_window_actors())
            this._watch(actor.meta_window);
    }

    disable() {
        global.display.disconnect(this._created);
        Main.overview.disconnect(this._overview);
        global.display.disconnect(this._activated);
        this._config.cancel();
        this._config = null;
        this._ungrab();
        this._grabs = null;
        for (const [win, ids] of this._watched)
            ids.forEach(id => win.disconnect(id));
        this._watched = null;
        this._placed = null;
    }

    _grab() {
        this._ungrab();
        for (const [action, accel] of Object.entries(readShortcuts())) {
            if (!accel)
                continue;
            const id = global.display.grab_accelerator(accel, Meta.KeyBindingFlags.NONE);
            if (id === Meta.KeyBindingAction.NONE)
                continue;
            Main.wm.allowKeybinding(Meta.external_binding_name_for_action(id), Shell.ActionMode.ALL);
            this._grabs.set(id, action);
        }
    }

    _ungrab() {
        for (const id of this._grabs.keys()) {
            Main.wm.allowKeybinding(Meta.external_binding_name_for_action(id), Shell.ActionMode.NONE);
            global.display.ungrab_accelerator(id);
        }
        this._grabs.clear();
    }

    _watch(win) {
        if (this._watched.has(win))
            return;
        const check = () => this._place(win);
        const ids = [
            win.connect('notify::title', check),
            win.connect('notify::wm-class', check),
            win.connect('unmanaged', () => {
                ids.forEach(id => win.disconnect(id));
                this._watched?.delete(win);
            }),
        ];
        this._watched.set(win, ids);
        check();
    }

    _setPassing(win, passing) {
        if (passing) {
            win.unmake_above();
            win.lower();
            return;
        }
        win.make_above();
        win.raise();
        if (!win.has_focus())
            Main.activateWindow(win);
    }

    _place(win) {
        if (!isOverlay(win))
            return;
        if (this._placed.has(win)) {
            this._setPassing(win, win.get_title().endsWith(PASS));
            return;
        }
        this._placed.add(win);
        const connector = win.get_title().slice(PREFIX.length).replace(PASS, '').trim();
        const index = global.backend.get_monitor_manager().get_monitor_for_connector(connector);
        win.make_above();
        win.stick();
        if (index >= 0)
            win.move_to_monitor(index);
        if (SHELL_MAJOR >= 49)
            win.maximize();
        else
            win.maximize(Meta.MaximizeFlags.BOTH);
        Main.activateWindow(win);
    }
}
