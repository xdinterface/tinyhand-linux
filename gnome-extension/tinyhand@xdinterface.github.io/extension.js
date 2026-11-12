import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import {Workspace} from 'resource:///org/gnome/shell/ui/workspace.js';
import {WorkspaceThumbnail} from 'resource:///org/gnome/shell/ui/workspaceThumbnail.js';
import {Extension, InjectionManager} from 'resource:///org/gnome/shell/extensions/extension.js';
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

function readConfig() {
    try {
        return new TextDecoder().decode(GLib.file_get_contents(configPath())[1]);
    } catch {
        return '';
    }
}

function readShortcuts(text) {
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
        this._fixing = new Set();
        this._created = global.display.connect('window-created', (_display, win) => this._watch(win));
        this._overview = Main.overview.connect('hidden', () => {
            for (const win of this._watched.keys()) {
                if (isOverlay(win) && !win.get_title().endsWith(PASS))
                    this._setPassing(win, false);
            }
        });
        this._injections = new InjectionManager();
        for (const proto of [Workspace.prototype, WorkspaceThumbnail.prototype]) {
            this._injections.overrideMethod(proto, '_isOverviewWindow', original => function (win) {
                return !isOverlay(win.get_meta_window?.() ?? win) && original.call(this, win);
            });
        }
        this._grabs = new Map();
        this._activated = global.display.connect('accelerator-activated', (_display, id) => {
            if (this._grabs.has(id))
                activate(this._grabs.get(id), Gio.DBusCallFlags.NONE);
        });
        this._config = Gio.File.new_for_path(configPath()).monitor_file(Gio.FileMonitorFlags.NONE, null);
        this._config.connect('changed', () => this._configure());
        this._running = false;
        this._busWatch = Gio.bus_watch_name(Gio.BusType.SESSION, APP_ID, Gio.BusNameWatcherFlags.NONE,
            () => this._setRunning(true), () => this._setRunning(false));
        this._configure();
        for (const actor of global.get_window_actors())
            this._watch(actor.meta_window);
    }

    disable() {
        global.display.disconnect(this._created);
        Main.overview.disconnect(this._overview);
        this._injections.clear();
        this._injections = null;
        Gio.bus_unwatch_name(this._busWatch);
        this._indicator?.destroy();
        this._indicator = null;
        global.display.disconnect(this._activated);
        this._config.cancel();
        this._config = null;
        this._ungrab();
        this._grabs = null;
        for (const [win, ids] of this._watched)
            ids.forEach(id => win.disconnect(id));
        this._watched = null;
        this._placed = null;
        this._fixing = null;
    }

    _setRunning(running) {
        this._running = running;
        this._configure();
    }

    _configure() {
        const text = readConfig();
        this._grab(readShortcuts(text));
        const wanted = this._running && !/^\s*tray\s*=\s*false/m.test(text);
        if (wanted && !this._indicator)
            this._addIndicator();
        else if (!wanted && this._indicator) {
            this._indicator.destroy();
            this._indicator = null;
        }
    }

    _addIndicator() {
        this._indicator = new PanelMenu.Button(0.5, 'TinyHand');
        this._icon = new St.Icon({style_class: 'system-status-icon'});
        this._indicator.add_child(this._icon);
        const item = (label, action) => {
            const entry = new PopupMenu.PopupMenuItem(label);
            entry.connect('activate', () => activate(action, Gio.DBusCallFlags.NONE));
            this._indicator.menu.addMenuItem(entry);
            return entry;
        };
        this._toggle = item('', 'toggle');
        item('Clear Drawing', 'clear');
        this._indicator.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        item('Settings…', 'settings');
        item('Quit TinyHand', 'quit');
        this._indicator.connect('destroy', () => (this._indicator = null));
        Main.panel.addToStatusArea(this.uuid, this._indicator);
        this._syncIndicator();
    }

    _syncIndicator() {
        if (!this._indicator)
            return;
        const on = [...this._watched.keys()].some(isOverlay);
        this._icon.gicon = Gio.icon_new_for_string(`${this.path}/tinyhand-${on ? 'on' : 'off'}-symbolic.svg`);
        this._toggle.label.text = on ? 'Turn Pointer Off' : 'Turn Pointer On';
    }

    _grab(shortcuts) {
        this._ungrab();
        for (const [action, accel] of Object.entries(shortcuts)) {
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
            win.connect('size-changed', () => this._keepMaximized(win)),
            win.connect('notify::minimized', () => this._keepMaximized(win)),
            win.connect('unmanaged', () => {
                ids.forEach(id => win.disconnect(id));
                this._watched?.delete(win);
                this._syncIndicator?.();
            }),
        ];
        this._watched.set(win, ids);
        check();
    }

    _maximize(win) {
        if (SHELL_MAJOR >= 49)
            win.maximize();
        else
            win.maximize(Meta.MaximizeFlags.BOTH);
    }

    _keepMaximized(win) {
        if (!this._placed?.has(win) || this._fixing?.has(win))
            return;
        const maximized = SHELL_MAJOR >= 49
            ? win.is_maximized()
            : win.get_maximized() === Meta.MaximizeFlags.BOTH;
        if (maximized && !win.minimized)
            return;
        this._fixing.add(win);
        GLib.idle_add(GLib.PRIORITY_DEFAULT, () => {
            this._fixing?.delete(win);
            if (this._placed?.has(win) && isOverlay(win)) {
                if (win.minimized)
                    win.unminimize();
                this._maximize(win);
            }
            return GLib.SOURCE_REMOVE;
        });
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
        this._syncIndicator();
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
        this._maximize(win);
        Main.activateWindow(win);
    }
}
