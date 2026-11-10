import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const APP_ID = 'io.github.xdinterface.TinyHand';
const OBJECT_PATH = '/io/github/xdinterface/TinyHand';
const PREFIX = 'TinyHand Overlay';

function isOverlay(win) {
    const title = win.get_title() ?? '';
    const app = win.get_gtk_application_id?.() ?? win.get_wm_class();
    return title.startsWith(PREFIX) && app === APP_ID;
}

function turnOff() {
    Gio.DBus.session.call(
        APP_ID, OBJECT_PATH, 'org.gtk.Actions', 'Activate',
        new GLib.Variant('(sava{sv})', ['off', [], {}]),
        null, Gio.DBusCallFlags.NO_AUTO_START, -1, null, null);
}

export default class TinyHandExtension extends Extension {
    enable() {
        this._watched = new Map();
        this._placed = new WeakSet();
        this._created = global.display.connect('window-created', (_display, win) => this._watch(win));
        this._overview = Main.overview.connect('showing', () => {
            if (global.get_window_actors().some(actor => isOverlay(actor.meta_window)))
                turnOff();
        });
        for (const actor of global.get_window_actors())
            this._watch(actor.meta_window);
    }

    disable() {
        global.display.disconnect(this._created);
        Main.overview.disconnect(this._overview);
        for (const [win, ids] of this._watched)
            ids.forEach(id => win.disconnect(id));
        this._watched = null;
        this._placed = null;
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

    _place(win) {
        if (this._placed.has(win) || !isOverlay(win))
            return;
        this._placed.add(win);
        const connector = win.get_title().slice(PREFIX.length).trim();
        const index = global.backend.get_monitor_manager().get_monitor_for_connector(connector);
        win.make_above();
        win.stick();
        if (index >= 0)
            win.move_to_monitor(index);
        if (Meta.MaximizeFlags)
            win.maximize(Meta.MaximizeFlags.BOTH);
        else
            win.maximize();
        Main.activateWindow(win);
    }
}
