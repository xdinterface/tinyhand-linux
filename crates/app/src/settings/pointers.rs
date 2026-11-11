use super::{Ctx, maker_dialog};
use crate::app::title;
use crate::hands;
use crate::maker;
use crate::toolbar::pointer_icon;
use adw::prelude::*;
use gtk::glib;
use std::rc::Rc;
use tinyhand_core::catalog::{self, DEFAULT_POINTER, DEFAULT_TOOLBAR, LASER, MAX_PINNED};
use tinyhand_core::config::Value;

const ICON: i32 = 44;

fn lazy_icon(id: &str, size: i32) -> gtk::Widget {
    if id == LASER || hands::has_thumbnail(id) {
        return pointer_icon(id, size);
    }
    let image = gtk::Image::new();
    image.set_pixel_size(size);
    let (weak, id) = (image.downgrade(), id.to_string());
    glib::idle_add_local_once(move || {
        if let Some(image) = weak.upgrade() {
            image.set_paintable(hands::thumbnail(&id).as_ref());
        }
    });
    image.upcast()
}

fn all_ids(ctx: &Ctx) -> Vec<String> {
    catalog::BUILTIN
        .iter()
        .map(|h| h.id.to_string())
        .chain(ctx.customs.borrow().iter().map(|h| h.id.clone()))
        .chain([LASER.to_string()])
        .collect()
}

fn name(ctx: &Ctx, id: &str) -> String {
    title(id, &ctx.customs.borrow())
}

fn save_toolbar(ctx: &Rc<Ctx>, ids: Vec<String>) {
    let value: Value = ids.iter().map(String::as_str).collect();
    ctx.set("toolbar", value, |c| c.toolbar = ids);
    ctx.rebuild();
}

fn unpin(ctx: &Rc<Ctx>, id: &str) {
    let mut ids = ctx.config.borrow().toolbar.clone();
    if ids.len() <= 1 {
        ctx.toast("Keep at least one pointer in the toolbar.");
        ctx.rebuild();
        return;
    }
    ids.retain(|p| p != id);
    save_toolbar(ctx, ids);
}

fn pin(ctx: &Rc<Ctx>, id: &str) -> bool {
    let mut ids = ctx.config.borrow().toolbar.clone();
    if ids.iter().any(|p| p == id) {
        return true;
    }
    if ids.len() >= MAX_PINNED {
        ctx.toast(&format!("The toolbar is full. Remove one to add {}.", name(ctx, id)));
        ctx.rebuild();
        return false;
    }
    ids.push(id.to_string());
    save_toolbar(ctx, ids);
    true
}

fn clear(container: &impl IsA<gtk::Widget>) {
    while let Some(child) = container.first_child() {
        child.unparent();
    }
}

fn delete(ctx: &Rc<Ctx>, id: String) {
    let dialog = adw::AlertDialog::new(
        Some(&format!("Delete {}?", name(ctx, &id))),
        Some("The image is removed from your computer."),
    );
    dialog.add_responses(&[("cancel", "Cancel"), ("delete", "Delete")]);
    dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
    let weak = Rc::downgrade(ctx);
    dialog.connect_response(Some("delete"), move |_, _| {
        let Some(ctx) = weak.upgrade() else { return };
        if let Err(err) = maker::delete(&id) {
            ctx.toast(&format!("Couldn't delete it: {err}"));
            return;
        }
        ctx.reload_customs();
        ctx.fresh();
        if ctx.config.borrow().pointer == id {
            ctx.set("pointer", DEFAULT_POINTER, |c| c.pointer = DEFAULT_POINTER.into());
        }
        let ids: Vec<String> = ctx.config.borrow().toolbar.iter().filter(|p| **p != id).cloned().collect();
        if ids.is_empty() {
            save_toolbar(&ctx, DEFAULT_TOOLBAR.map(String::from).to_vec());
        } else {
            save_toolbar(&ctx, ids);
        }
    });
    dialog.present(Some(&ctx.window));
}

pub fn page(ctx: &Rc<Ctx>) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::new();

    let pinned_group = adw::PreferencesGroup::new();
    pinned_group.set_title("In your toolbar");
    pinned_group.set_description(Some("Drag to reorder, click to remove. Keys 1 to 9 pick them while the pointer is on."));
    let reset = gtk::Button::with_label("Reset");
    reset.add_css_class("flat");
    let weak = Rc::downgrade(ctx);
    reset.connect_clicked(move |_| {
        if let Some(ctx) = weak.upgrade() {
            save_toolbar(&ctx, DEFAULT_TOOLBAR.map(String::from).to_vec());
        }
    });
    pinned_group.set_header_suffix(Some(&reset));
    let pinned = gtk::FlowBox::new();
    pinned.set_selection_mode(gtk::SelectionMode::None);
    pinned.set_max_children_per_line(MAX_PINNED as u32);
    pinned.add_css_class("card");
    pinned_group.add(&pinned);
    page.add(&pinned_group);

    let all_group = adw::PreferencesGroup::new();
    all_group.set_title("Pointers");
    all_group.set_description(Some("Click a pointer to add it to the toolbar."));
    let grid = gtk::FlowBox::new();
    grid.set_selection_mode(gtk::SelectionMode::None);
    grid.set_max_children_per_line(10);
    grid.set_min_children_per_line(4);
    grid.set_homogeneous(true);
    all_group.add(&grid);
    page.add(&all_group);

    let own_group = adw::PreferencesGroup::new();
    own_group.set_title("Your hands");
    own_group.set_description(Some("Turn any idea into a hand with ChatGPT or another image tool."));
    let make = gtk::Button::with_label("Make your own…");
    make.add_css_class("flat");
    let weak = Rc::downgrade(ctx);
    make.connect_clicked(move |_| {
        if let Some(ctx) = weak.upgrade() {
            maker_dialog::open(&ctx);
        }
    });
    own_group.set_header_suffix(Some(&make));
    let own = gtk::ListBox::new();
    own.add_css_class("boxed-list");
    own.set_selection_mode(gtk::SelectionMode::None);
    own_group.add(&own);
    page.add(&own_group);

    let fill = {
        let weak = Rc::downgrade(ctx);
        move || {
            let Some(ctx) = weak.upgrade() else { return };
            let toolbar = ctx.config.borrow().toolbar.clone();

            pinned.remove_all();
            for id in &toolbar {
                let button = gtk::Button::new();
                button.set_child(Some(&lazy_icon(id, ICON)));
                button.add_css_class("flat");
                button.set_tooltip_text(Some(&format!("Remove {} from the toolbar", name(&ctx, id))));
                let (weak, removed) = (Rc::downgrade(&ctx), id.clone());
                button.connect_clicked(move |_| {
                    if let Some(ctx) = weak.upgrade() {
                        unpin(&ctx, &removed);
                    }
                });
                let drag = gtk::DragSource::new();
                drag.set_actions(gtk::gdk::DragAction::MOVE);
                drag.set_content(Some(&gtk::gdk::ContentProvider::for_value(&id.to_value())));
                drag.connect_drag_begin(|drag, _| {
                    if let Some(widget) = drag.widget() {
                        drag.set_icon(Some(&gtk::WidgetPaintable::new(Some(&widget))), ICON / 2, ICON / 2);
                    }
                });
                button.add_controller(drag);
                let drop = gtk::DropTarget::new(String::static_type(), gtk::gdk::DragAction::MOVE);
                let (weak, onto) = (Rc::downgrade(&ctx), id.clone());
                drop.connect_drop(move |_, value, _, _| {
                    let (Some(ctx), Ok(id)) = (weak.upgrade(), value.get::<String>()) else {
                        return false;
                    };
                    let mut ids = ctx.config.borrow().toolbar.clone();
                    let moved = tinyhand_core::config::move_pinned(&mut ids, &id, &onto);
                    if moved {
                        save_toolbar(&ctx, ids);
                    }
                    moved
                });
                button.add_controller(drop);
                pinned.append(&button);
            }

            grid.remove_all();
            for id in all_ids(&ctx) {
                let toggle = gtk::ToggleButton::new();
                toggle.set_child(Some(&lazy_icon(&id, ICON)));
                toggle.add_css_class("flat");
                toggle.set_active(toolbar.contains(&id));
                toggle.set_tooltip_text(Some(&name(&ctx, &id)));
                let weak = Rc::downgrade(&ctx);
                toggle.connect_toggled(move |toggle| {
                    let Some(ctx) = weak.upgrade() else { return };
                    if toggle.is_active() {
                        pin(&ctx, &id);
                    } else {
                        unpin(&ctx, &id);
                    }
                });
                grid.append(&toggle);
            }

            clear(&own);
            let customs = ctx.customs.borrow().clone();
            own.set_visible(!customs.is_empty());
            for hand in customs {
                let row = adw::ActionRow::new();
                row.set_title(&hand.name);
                row.add_prefix(&pointer_icon(&hand.id, 32));
                let remove = gtk::Button::from_icon_name("user-trash-symbolic");
                remove.add_css_class("flat");
                remove.set_valign(gtk::Align::Center);
                remove.set_tooltip_text(Some(&format!("Delete {}", hand.name)));
                let weak = Rc::downgrade(&ctx);
                remove.connect_clicked(move |_| {
                    if let Some(ctx) = weak.upgrade() {
                        delete(&ctx, hand.id.clone());
                    }
                });
                row.add_suffix(&remove);
                own.append(&row);
            }
        }
    };
    fill();
    ctx.on_rebuild(fill);
    page
}

pub fn add_to_toolbar(ctx: &Rc<Ctx>, id: &str) -> bool {
    pin(ctx, id)
}
