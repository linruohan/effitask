mod globals;
mod preferences;

pub use globals::preferences::get as preferences;
pub use globals::tasks::get as tasks;

use globals::tasks::add as add_task;
use preferences::Preferences;

use adw::prelude::*;
use relm4::ComponentController as _;

pub const NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
enum Page {
    Inbox = 0,
    Projects,
    Contexts,
    Agenda,
    Flag,
    Done,
    Search,
    Tags,
}

impl From<u32> for Page {
    fn from(n: u32) -> Self {
        match n {
            0 => Page::Inbox,
            1 => Page::Projects,
            2 => Page::Contexts,
            3 => Page::Tags,
            4 => Page::Agenda,
            5 => Page::Flag,
            6 => Page::Done,
            7 => Page::Search,
            _ => panic!("Invalid page {n}"),
        }
    }
}

impl From<Page> for u32 {
    fn from(page: Page) -> u32 {
        unsafe { std::mem::transmute(page) }
    }
}

#[derive(Clone, Debug)]
pub enum Msg {
    Adding,
    Add(String),
    Complete(Box<crate::tasks::Task>),
    Edit(Box<crate::tasks::Task>),
    EditCancel,
    EditDone(Box<crate::tasks::Task>),
    Find,
    Help,
    Refresh,
    Search(String),
}

pub struct Model {
    agenda: relm4::Controller<crate::agenda::Model>,
    config: todo_txt::Config,
    contexts: relm4::Controller<crate::widgets::tags::Model>,
    done: relm4::Controller<crate::done::Model>,
    edit: relm4::Controller<crate::edit::Model>,
    flag: relm4::Controller<crate::flag::Model>,
    inbox: relm4::Controller<crate::inbox::Model>,
    logger: relm4::Controller<crate::logger::Model>,
    projects: relm4::Controller<crate::widgets::tags::Model>,
    shortcuts: gtk::ShortcutsWindow,
    search: relm4::Controller<crate::search::Model>,
    tags: relm4::Controller<crate::widgets::tags::Model>,
}

impl Model {
    fn load_style(&self) {
        let css = gtk::CssProvider::new();
        css.load_from_resource(&self.stylesheet());

        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().unwrap(),
            &css,
            0,
        );
    }

    fn stylesheet(&self) -> String {
        let mut stylesheet = "style_light.css";

        if let Ok(theme) = std::env::var("GTK_THEME") {
            if theme.ends_with(":dark") {
                stylesheet = "style_dark.css";
            }
        } else if let Some(setting) = gtk::Settings::default() {
            if setting.is_gtk_application_prefer_dark_theme() {
                stylesheet = "style_dark.css";
            }
        }

        format!("/txt/todo/effitask/{stylesheet}")
    }

    fn add_tab_widgets(&self, notebook: &gtk::Notebook) {
        let n = notebook.n_pages();

        for x in 0..n {
            let page = notebook.nth_page(Some(x)).unwrap();
            let widget = self.tab_widget(x);

            notebook.set_tab_label(&page, Some(&widget));
        }
    }

    fn tab_widget(&self, n: u32) -> gtk::Box {
        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
        vbox.set_homogeneous(false);

        let title = match n.into() {
            Page::Inbox => "inbox",
            Page::Projects => "projects",
            Page::Contexts => "contexts",
            Page::Agenda => "agenda",
            Page::Flag => "flag",
            Page::Done => "done",
            Page::Search => "search",
            Page::Tags => "tags",
        };

        let image = gtk::Image::from_icon_name(title);
        image.set_icon_size(gtk::IconSize::Large);
        vbox.append(&image);

        let label = gtk::Label::new(Some(title));
        vbox.append(&label);

        vbox
    }

    fn add(&mut self, widgets: &ModelWidgets, text: &str) {
        match add_task(text) {
            Ok(_) => self.update_tasks(widgets),
            Err(err) => log::error!("Unable to create task: '{err}'"),
        }

        widgets.add_popover.popdown();
    }

    fn complete(&mut self, widgets: &ModelWidgets, task: &crate::tasks::Task) {
        let id = task.id;
        let mut list = tasks();

        if let Some(ref mut t) = list.tasks.get_mut(id) {
            if t.finished {
                t.uncomplete();
            } else {
                t.complete();
            }
        } else {
            return;
        }

        let t = list.tasks[id].clone();

        if t.finished {
            if let Some(ref recurrence) = t.recurrence {
                let due = if recurrence.strict && t.due_date.is_some() {
                    t.due_date.unwrap()
                } else {
                    crate::date::today()
                };

                let mut new: crate::tasks::Task = t.clone();
                new.uncomplete();
                new.create_date = Some(crate::date::today());
                new.due_date = Some(recurrence.clone() + due);

                if let Some(threshold_date) = t.threshold_date {
                    new.threshold_date = Some(recurrence.clone() + threshold_date);
                }

                list.push(new);
            }
        }

        match list.write() {
            Ok(_) => {
                if list.tasks[id].finished {
                    log::info!("Task done");
                } else {
                    log::info!("Task undone");
                }
            }
            Err(err) => log::error!("Unable to save tasks: {err}"),
        };

        self.update_tasks(widgets);
    }

    fn edit(&mut self, task: &crate::tasks::Task) {
        self.edit
            .emit(crate::edit::MsgInput::Set(Box::new(task.clone())));
        self.edit.widget().set_visible(true);
    }

    fn save(&mut self, widgets: &ModelWidgets, task: &crate::tasks::Task) {
        let id = task.id;
        let mut list = tasks();

        if list.tasks.get_mut(id).is_some() {
            list.tasks[id] = task.clone();
        }

        match list.write() {
            Ok(_) => (),
            Err(err) => log::error!("Unable to save tasks: {err}"),
        };

        log::info!("Task updated");

        self.update_tasks(widgets);
        self.edit.widget().set_visible(false);
    }

    fn search(&self, widgets: &ModelWidgets, query: &str) {
        if query.is_empty() {
            widgets.notebook.set_current_page(Some(Page::Inbox.into()));
            self.search.widget().set_visible(false);
        } else {
            self.search.widget().set_visible(true);
            widgets.notebook.set_current_page(Some(Page::Search.into()));
        }

        self.search
            .emit(crate::search::MsgInput::UpdateFilter(query.to_string()));
    }

    fn update_tasks(&self, widgets: &ModelWidgets) {
        let list = crate::tasks::List::from_files(&self.config.todo_file, &self.config.done_file);
        globals::tasks::replace(list);

        globals::preferences::replace(crate::application::Preferences {
            defered: widgets.defered_button.is_active(),
            done: widgets.done_button.is_active(),
        });

        self.agenda.sender().emit(crate::agenda::MsgInput::Update);
        self.contexts
            .sender()
            .emit(crate::widgets::tags::MsgInput::Update);
        self.done.sender().emit(crate::done::Msg::Update);
        self.projects
            .sender()
            .emit(crate::widgets::tags::MsgInput::Update);
        self.flag.sender().emit(crate::flag::Msg::Update);
        self.inbox.sender().emit(crate::inbox::Msg::Update);
        self.search.sender().emit(crate::search::MsgInput::Update);
        self.tags
            .sender()
            .emit(crate::widgets::tags::MsgInput::Update);

        log::info!("Tasks reloaded");
    }

    fn watch(&self, sender: relm4::ComponentSender<Self>) {
        use notify::Watcher as _;

        let mut watcher = notify::recommended_watcher(move |res| match res {
            Ok(_) => {
                sender.input(Msg::Refresh);
            }
            Err(e) => log::warn!("watch error: {e:?}"),
        })
        .unwrap();

        log::debug!("watching {} for changes", self.config.todo_file);

        if let Err(err) = watcher.watch(
            std::path::PathBuf::from(self.config.todo_file.clone()).as_path(),
            notify::RecursiveMode::Recursive,
        ) {
            log::warn!("Unable to setup hot reload: {err}");
        }
    }

    fn shortcuts(window: &adw::ApplicationWindow, sender: relm4::ComponentSender<Self>) {
        static SHORTCUTS: &[(&str, Msg)] = &[
            ("<Control>A", Msg::Adding),
            ("<Control>F", Msg::Find),
            ("F3", Msg::Find),
            ("<Control>R", Msg::Refresh),
            ("F5", Msg::Refresh),
        ];

        let controller = gtk::ShortcutController::new();
        controller.set_scope(gtk::ShortcutScope::Global);

        for (trigger, msg) in SHORTCUTS {
            let trigger = gtk::ShortcutTrigger::parse_string(trigger);
            let callback = gtk::CallbackAction::new(gtk::glib::clone!(
                #[strong]
                sender,
                #[strong]
                msg,
                move |_, _| {
                    sender.input(msg.clone());
                    gtk::glib::Propagation::Stop
                }
            ));

            let shortcut = gtk::Shortcut::new(trigger, Some(callback));
            controller.add_shortcut(shortcut);
        }

        window.add_controller(controller);
    }
}

#[relm4::component(pub)]
impl relm4::Component for Model {
    type CommandOutput = ();
    type Init = todo_txt::Config;
    type Input = Msg;
    type Output = ();

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let logger = crate::logger::Model::builder().launch(()).detach();

        let agenda = crate::agenda::Model::builder()
            .launch(crate::date::today())
            .forward(sender.input_sender(), |output| match output {
                crate::agenda::MsgOutput::Complete(task) => Msg::Complete(task),
                crate::agenda::MsgOutput::Edit(task) => Msg::Edit(task),
            });

        let contexts = crate::widgets::tags::Model::builder()
            .launch(crate::widgets::tags::Type::Contexts)
            .forward(sender.input_sender(), |output| match output {
                crate::widgets::tags::MsgOutput::Complete(task) => Msg::Complete(task),
                crate::widgets::tags::MsgOutput::Edit(task) => Msg::Edit(task),
            });

        let done =
            crate::done::Model::builder()
                .launch(())
                .forward(sender.input_sender(), |output| match output {
                    crate::widgets::task::MsgOutput::Complete(task) => Msg::Complete(task),
                    crate::widgets::task::MsgOutput::Edit(task) => Msg::Edit(task),
                });

        let edit = crate::edit::Model::builder()
            .launch(crate::tasks::Task::new())
            .forward(sender.input_sender(), |output| match output {
                crate::edit::MsgOutput::Cancel => Msg::EditCancel,
                crate::edit::MsgOutput::Done(task) => Msg::EditDone(task),
            });

        let flag =
            crate::flag::Model::builder()
                .launch(())
                .forward(sender.input_sender(), |output| match output {
                    crate::widgets::task::MsgOutput::Complete(task) => Msg::Complete(task),
                    crate::widgets::task::MsgOutput::Edit(task) => Msg::Edit(task),
                });

        let inbox =
            crate::inbox::Model::builder()
                .launch(())
                .forward(sender.input_sender(), |output| match output {
                    crate::widgets::task::MsgOutput::Complete(task) => Msg::Complete(task),
                    crate::widgets::task::MsgOutput::Edit(task) => Msg::Edit(task),
                });

        let projects = crate::widgets::tags::Model::builder()
            .launch(crate::widgets::tags::Type::Projects)
            .forward(sender.input_sender(), |output| match output {
                crate::widgets::tags::MsgOutput::Complete(task) => Msg::Complete(task),
                crate::widgets::tags::MsgOutput::Edit(task) => Msg::Edit(task),
            });

        let search = crate::search::Model::builder()
            .launch(String::new())
            .forward(sender.input_sender(), |output| match output {
                crate::widgets::task::MsgOutput::Complete(task) => Msg::Complete(task),
                crate::widgets::task::MsgOutput::Edit(task) => Msg::Edit(task),
            });

        let tags = crate::widgets::tags::Model::builder()
            .launch(crate::widgets::tags::Type::Hashtags)
            .forward(sender.input_sender(), |output| match output {
                crate::widgets::tags::MsgOutput::Complete(task) => Msg::Complete(task),
                crate::widgets::tags::MsgOutput::Edit(task) => Msg::Edit(task),
            });

        let builder = gtk::Builder::from_resource("/txt/todo/effitask/shortcuts.ui");
        let shortcuts = builder.object("shortcuts").unwrap();

        let model = Self {
            agenda,
            config: init,
            contexts,
            done,
            edit,
            flag,
            inbox,
            logger,
            projects,
            search,
            shortcuts,
            tags,
        };

        let widgets = view_output!();

        model.load_style();
        model.add_tab_widgets(&widgets.notebook);
        model.update_tasks(&widgets);
        model.search.widget().set_visible(false);
        model.watch(sender.clone());

        Self::shortcuts(&root, sender);

        relm4::ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        _: relm4::ComponentSender<Self>,
        _: &Self::Root,
    ) {
        match msg {
            Msg::Add(task) => self.add(widgets, &task),
            Msg::Adding => widgets.add_popover.popup(),
            Msg::Complete(task) => self.complete(widgets, &task),
            Msg::EditCancel => self.edit.widget().set_visible(false),
            Msg::EditDone(task) => self.save(widgets, &task),
            Msg::Edit(task) => self.edit(&task),
            Msg::Find => {
                widgets.search.grab_focus();
            }
            Msg::Help => self.shortcuts.present(),
            Msg::Refresh => self.update_tasks(widgets),
            Msg::Search(query) => self.search(widgets, &query),
        }
    }

    view! {
        adw::ApplicationWindow {
            set_title: NAME.into(),
            set_decorated: false,
            #[wrap(Some)]
            set_content = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                adw::HeaderBar {
                    set_title_widget: Some(&gtk::Label::new(NAME.into())),

                    pack_start = &gtk::Button {
                        set_icon_name: "view-refresh",
                        set_tooltip_text: "Refresh".into(),

                        connect_clicked => Msg::Refresh,
                    },
                    pack_start = &gtk::MenuButton {
                        set_icon_name: "list-add",
                        set_tooltip_text: "Add".into(),
                        #[wrap(Some)]
                        #[name = "add_popover"]
                        set_popover = &gtk::Popover {
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                gtk::Label {
                                    set_text: "Create a new task +project @context due:2050-01-01",
                                },
                                gtk::Entry {
                                    connect_activate[sender] => move |this| {
                                        sender.input(Msg::Add(this.text().to_string()));
                                        println!("Current text: {}", this.text());
                                        this.set_text("");
                                    }
                                },

                            },
                        },
                    },
                    pack_start = &gtk::MenuButton {
                        set_icon_name: "preferences-system",
                        set_tooltip_text: "Preferences".into(),
                        #[wrap(Some)]
                        set_popover = &gtk::Popover {
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                #[name = "defered_button"]
                                gtk::CheckButton {
                                    set_label: Some("Display defered tasks"),

                                    connect_toggled => Msg::Refresh,
                                },
                                #[name = "done_button"]
                                gtk::CheckButton {
                                    set_label: Some("Display done tasks"),
                                    connect_toggled => Msg::Refresh,
                                },
                            },
                        },
                    },
                    pack_start = &gtk::Button {
                        set_icon_name: "help-about",
                        set_tooltip_text: "Help".into(),

                        connect_clicked => Msg::Help,
                    },

                    pack_end = model.logger.widget(),
                    #[name = "search"]
                    pack_end = &gtk::SearchEntry {
                        connect_search_changed[sender] => move |this| {
                            sender.input(Msg::Search(this.text().to_string()));
                        },
                    },
                },
                gtk::Paned {
                    set_hexpand: true,
                    set_vexpand: true,
                    set_orientation: gtk::Orientation::Horizontal,
                    set_wide_handle: true,

                    #[wrap(Some)]
                    #[name = "notebook"]
                    set_start_child = &gtk::Notebook {
                        set_tab_pos: gtk::PositionType::Left,

                        append_page: (model.inbox.widget(), None::<&gtk::Label>),
                        append_page: (model.projects.widget(), None::<&gtk::Label>),
                        append_page: (model.contexts.widget(), None::<&gtk::Label>),
                        append_page: (model.tags.widget(), None::<&gtk::Label>),
                        append_page: (model.agenda.widget(), None::<&gtk::Label>),
                        append_page: (model.flag.widget(), None::<&gtk::Label>),
                        append_page: (model.done.widget(), None::<&gtk::Label>),
                        append_page: (model.search.widget(), None::<&gtk::Label>),
                    },
                    #[wrap(Some)]
                    set_end_child = model.edit.widget(),
                },
            },
            connect_close_request => move |_| {
                relm4::main_application().quit();
                gtk::glib::Propagation::Stop
            },
        }
    }
}
