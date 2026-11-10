use super::{Ctx, pointers};
use crate::maker::{self, Prepared};
use adw::prelude::*;
use gtk::{gdk, gio, glib};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use tinyhand_core::custom;
use tinyhand_core::handmaker;
use tinyhand_core::paths;

struct Draft {
    prepared: RefCell<Option<Prepared>>,
    picture: gtk::Picture,
    status: gtk::Label,
    name: adw::EntryRow,
    add: gtk::Button,
}

impl Draft {
    fn load(&self, path: &Path) {
        match maker::prepare(path) {
            Ok(prepared) => {
                self.picture.set_paintable(Some(&prepared.texture));
                self.picture.set_visible(true);
                self.status.set_label(
                    "It should show the back of the hand, like the ones in TinyHand. If you see the palm, try another image.",
                );
                self.status.remove_css_class("error");
                let count = custom::load(&paths::custom_hands_dir()).len();
                let stem = path.file_stem().unwrap_or_default().to_string_lossy();
                self.name.set_text(&custom::suggested_name(&stem, count));
                self.prepared.replace(Some(prepared));
                self.add.set_sensitive(true);
            }
            Err(problem) => {
                self.prepared.take();
                self.picture.set_visible(false);
                self.status.set_label(problem);
                self.status.add_css_class("error");
                self.add.set_sensitive(false);
            }
        }
    }
}

fn group(title: &str, description: &str) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::new();
    group.set_title(title);
    group.set_description(Some(description));
    group
}

pub fn open(ctx: &Rc<Ctx>) {
    let dialog = adw::Dialog::new();
    dialog.set_title("Make your own hand");
    dialog.set_content_width(560);
    dialog.set_content_height(720);
    let page = adw::PreferencesPage::new();

    let describe = group("1. Describe it", "What should your hand look like?");
    let idea = adw::EntryRow::new();
    idea.set_title("Knitted wool in mustard yellow");
    describe.add(&idea);
    page.add(&describe);

    let make = group(
        "2. Make the image",
        "Open it in ChatGPT, or copy the prompt for another image tool. Download the result as a PNG with a transparent background.",
    );
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let chatgpt = gtk::Button::with_label("Open in ChatGPT");
    chatgpt.add_css_class("suggested-action");
    chatgpt.add_css_class("pill");
    let copy = gtk::Button::with_label("Copy prompt");
    copy.add_css_class("pill");
    buttons.append(&chatgpt);
    buttons.append(&copy);
    make.add(&buttons);
    page.add(&make);
    chatgpt.connect_clicked(glib::clone!(
        #[weak]
        idea,
        move |button| {
            let prompt = handmaker::prompt(&idea.text());
            let url = format!("https://chatgpt.com/?q={}", glib::Uri::escape_string(&prompt, None, false));
            let window = button.root().and_downcast::<gtk::Window>();
            gtk::UriLauncher::new(&url).launch(window.as_ref(), gio::Cancellable::NONE, |_| {});
        }
    ));
    copy.connect_clicked(glib::clone!(
        #[weak]
        idea,
        move |button| {
            button.clipboard().set_text(&handmaker::prompt(&idea.text()));
            button.set_label("Copied");
        }
    ));

    let add_group = group("3. Add it", "Drop the PNG here, or choose it.");
    let choose = gtk::Button::with_label("Choose image…");
    choose.add_css_class("pill");
    choose.set_halign(gtk::Align::Start);
    add_group.add(&choose);
    let picture = gtk::Picture::new();
    picture.set_content_fit(gtk::ContentFit::Contain);
    picture.set_size_request(-1, 260);
    picture.set_visible(false);
    picture.set_margin_top(12);
    add_group.add(&picture);
    let status = gtk::Label::new(None);
    status.set_wrap(true);
    status.set_xalign(0.0);
    status.add_css_class("dim-label");
    status.set_margin_top(8);
    add_group.add(&status);
    let names = adw::PreferencesGroup::new();
    let name = adw::EntryRow::new();
    name.set_title("Name");
    name.set_max_length(16);
    names.add(&name);
    let add = gtk::Button::with_label("Add hand");
    add.add_css_class("suggested-action");
    add.add_css_class("pill");
    add.set_halign(gtk::Align::Center);
    add.set_margin_top(12);
    add.set_sensitive(false);
    page.add(&add_group);
    page.add(&names);
    let footer = adw::PreferencesGroup::new();
    footer.add(&add);
    page.add(&footer);

    let draft = Rc::new(Draft { prepared: RefCell::default(), picture, status, name, add: add.clone() });

    choose.connect_clicked(glib::clone!(
        #[strong]
        draft,
        move |button| {
            let filter = gtk::FileFilter::new();
            filter.add_mime_type("image/png");
            filter.set_name(Some("PNG images"));
            let filters = gio::ListStore::new::<gtk::FileFilter>();
            filters.append(&filter);
            let chooser = gtk::FileDialog::builder().title("Pick the PNG of your hand").filters(&filters).build();
            let window = button.root().and_downcast::<gtk::Window>();
            let draft = draft.clone();
            chooser.open(window.as_ref(), gio::Cancellable::NONE, move |file| {
                if let Some(path) = file.ok().and_then(|f| f.path()) {
                    draft.load(&path);
                }
            });
        }
    ));

    let drop = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
    drop.connect_drop(glib::clone!(
        #[strong]
        draft,
        move |_, value, _, _| {
            let path = value.get::<gdk::FileList>().ok().and_then(|l| l.files().first().and_then(|f| f.path()));
            match path {
                Some(path) => {
                    draft.load(&path);
                    true
                }
                None => false,
            }
        }
    ));
    page.add_controller(drop);

    let weak = Rc::downgrade(ctx);
    add.connect_clicked(glib::clone!(
        #[strong]
        draft,
        #[weak]
        dialog,
        move |_| {
            let Some(ctx) = weak.upgrade() else { return };
            let result = draft.prepared.borrow().as_ref().map(|p| maker::save(p, &draft.name.text()));
            match result {
                Some(Ok(hand)) => {
                    ctx.reload_customs();
                    let pinned = pointers::add_to_toolbar(&ctx, &hand.id);
                    ctx.rebuild();
                    ctx.toast(&if pinned {
                        format!("Added {} to your toolbar", hand.name)
                    } else {
                        format!("Added {}", hand.name)
                    });
                    dialog.close();
                }
                Some(Err(problem)) => {
                    draft.status.set_label(problem);
                    draft.status.add_css_class("error");
                }
                None => {}
            }
        }
    ));

    let view = adw::ToolbarView::new();
    view.add_top_bar(&adw::HeaderBar::new());
    view.set_content(Some(&page));
    dialog.set_child(Some(&view));
    dialog.present(Some(&ctx.window));
}
